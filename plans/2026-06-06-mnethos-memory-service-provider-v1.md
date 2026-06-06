# Mnethos Memory Service — First-Class Auth Provider

## Objective

Replace the hand-edited `memory.json` ({server_url, token}) mechanism for long-term
memory authorization with a proper, first-class **provider** named **"Mnethos
Memory Service"** that the user authenticates via the existing `provider login`
flow (prompting for an API key). The memory token must live in the standard
credential store (`.credentials.json`), and the memory tools (`remember` +
`mem_search`) must be gated on that provider being configured — reusing the
existing provider machinery (the `mnethos_services` / `ContextEngine` precedent),
with no parallel/bespoke mechanisms and no hacks.

Expected outcomes:
- `mnethos provider login mnethos_memory` prompts "Enter your Mnethos Memory
  Service API key", stores the credential, and does NOT try to set a chat model.
- `mnethos provider login` (no arg) lists the memory provider alongside others.
- `mnethos provider logout mnethos_memory` removes it (works for free via the
  generic logout path).
- The provider is excluded from chat/`:model` selection (it is non-LLM).
- The memory client reads its `server_url` + bearer token from the
  provider/credential layer; `memory.json` is removed.
- Memory tools are offered iff the `mnethos_memory` provider has a credential.

## Background / Grounding (current architecture)

- Memory client today: `crates/forge_repo/src/memory.rs:30-48` reads
  `<base_path>/memory.json` into `MemoryConfig { server_url, token }` per call;
  attaches `authorization: Bearer <token>` (`memory.rs:63-65,82,113`).
- Gating today: `crates/forge_app/src/tool_registry.rs:279-284` requires the
  `MNETHOS_MEMORY` env var AND `memory.json` to exist.
- Provider model: `ProviderType` enum (`crates/forge_domain/src/provider.rs:17-23`)
  = `Llm` (default) + `ContextEngine`. `Provider<T>.is_configured()` ==
  `credential.is_some()` (`provider.rs:267-269`).
- Non-LLM precedent: `mnethos_services` provider
  (`crates/forge_repo/src/provider/provider.json:1297-1303`, `provider_type:
  "context_engine"`, `auth_methods:["api_key"]`, no `response_type`/`models`),
  `ProviderId::MNETHOS_SERVICES` (`provider.rs:67`), token read at runtime via
  `get_credential(&ProviderId::MNETHOS_SERVICES)`
  (`crates/forge_services/src/context_engine.rs:98-120`).
- Login flow: `handle_provider_login` (`ui.rs:1123-1156`) → `configure_provider`
  (`ui.rs:3356-3420`) → `handle_api_key_input` (`ui.rs:2979`, the "Enter your
  <X> API key" prompt) → `complete_provider_auth`
  (`crates/forge_services/src/provider_auth.rs:81`) → `ApiKeyStrategy::complete`
  (`crates/forge_infra/src/auth/strategy.rs:41-53`) → `upsert_credential`.
  **Confirmed: this path performs NO model fetch / LLM validation** — it only
  persists an `AuthCredential`.
- The only LLM-specific login step is `finalize_provider_activation`
  (`ui.rs:3628-3697`), which sets the chat default + selects a model.
- Chat/`:model` provider picker `select_provider` (`ui.rs:3506-3520`) already
  filters to `ProviderType::Llm`, so a non-LLM provider is auto-excluded.
- Integration seam: `ForgeRepo<F>` implements BOTH `ProviderRepository`
  (`get_credential`/`get_provider`, `forge_repo.rs:185-212`) AND
  `MemoryRepository` (delegates to `self.memory_repo`,
  `forge_repo.rs:530-546`) — the natural place to resolve the credential + URL
  and pass them into the gRPC client.

## Key Design Decisions

1. **New `ProviderType::Memory` variant** (not reuse of `ContextEngine`) — keeps
   the category self-documenting and guarantees exclusion from the `Llm`-only
   chat picker.
2. **`server_url` source of truth = provider.json `url`** for `mnethos_memory`
   (the production AWM gRPC endpoint `https://awm.mnethos.com:8084`, stable, with
   Let's Encrypt TLS so standard webpki roots work). Login prompts ONLY for the
   API key (no URL prompt).
3. **`ForgeMemoryRepository` becomes a stateless gRPC client** taking
   `server_url` + `token` as arguments; the credential/URL resolution and the
   "unconfigured ⇒ no-op" decision move up to `ForgeRepo`'s `MemoryRepository`
   impl (where the credential store is in scope). `memory.json` is removed.
4. **Gating becomes credential-based**: memory tools are offered iff a credential
   for `ProviderId::MNETHOS_MEMORY` exists — removing the `MNETHOS_MEMORY` env +
   `memory.json` checks (single source of truth, provider-consistent). This is
   the clean replacement; keeping the old env/file checks alongside would be the
   "костыль" the requester explicitly forbids.
5. **Reuse, don't fork**: logout, list, credential persistence, and the api-key
   prompt are all reused unchanged; the only new UI code is a one-line guard to
   skip `finalize_provider_activation` for non-LLM providers.

## Implementation Plan

### Domain layer (forge_domain)

- [ ] Task 1. Add a `Memory` variant to `ProviderType`
  (`crates/forge_domain/src/provider.rs:17-23`). It serializes as `"memory"`
  (snake_case via existing serde/strum derives). Rationale: a distinct,
  self-documenting non-LLM category that the `Llm`-only chat picker excludes.
- [ ] Task 2. Add `pub const MNETHOS_MEMORY: ProviderId =
  ProviderId(Cow::Borrowed("mnethos_memory"))` (`provider.rs:67` neighborhood),
  register it in `built_in_providers()` (`provider.rs:88-125`), add the
  `"mnethos_memory" => ProviderId::MNETHOS_MEMORY` arm to `FromStr`
  (`provider.rs:179-216`), and add the display-name arm
  `"mnethos_memory" => "Mnethos Memory Service".to_string()` in `display_name()`
  (`provider.rs:136-158`). Rationale: the requested brand name and a stable,
  parseable id consistent with all other built-ins.
- [ ] Task 3. Audit every match/usage of `ProviderType` for exhaustiveness after
  adding the variant (e.g. `crates/forge_services/src/provider_service.rs`,
  `crates/forge_main/src/model.rs`, `crates/forge_repo/src/provider/*`). Most
  sites use equality comparisons (safe); fix any exhaustive `match` that would
  fail to compile. Rationale: prevent silent breakage from the new enum case.

### Provider registry (provider.json + schema)

- [ ] Task 4. Add the `mnethos_memory` entry to
  `crates/forge_repo/src/provider/provider.json` (next to `mnethos_services`),
  shaped as a non-LLM provider: `{"id":"mnethos_memory","provider_type":"memory",
  "url":"https://awm.mnethos.com:8084","auth_methods":["api_key"]}` — NO
  `response_type`, NO `models`. Rationale: makes the provider discoverable by
  `get_providers()`/`get_provider()` and supplies the gRPC `server_url` as the
  single source of truth.
- [ ] Task 5. Add `"memory"` to the `provider_type` enum in
  `forge.schema.json:784` (and any generated/checked-in schema) so config
  validation accepts the new value. Rationale: keep the JSON schema authoritative
  and green.

### Login UX guard (forge_main)

- [ ] Task 6. In `handle_provider_login` (`crates/forge_main/src/ui.rs:1146-1155`),
  after `configure_provider` returns the configured provider, branch on
  `provider.provider_type`: for non-`Llm` providers, print a success title
  (e.g. "Mnethos Memory Service configured") and return WITHOUT calling
  `finalize_provider_activation`. Rationale: memory has no chat models and must
  not become the chat default; this mirrors the existing `MNETHOS_SERVICES`
  short-circuit philosophy and is the only new UI logic required. (Verify
  `configure_provider` already works unchanged for `mnethos_memory`: it is not
  `MNETHOS_SERVICES`, single `ApiKey` method ⇒ direct `handle_api_key_input`.)
- [ ] Task 7. Confirm (and add a test) that the memory provider is EXCLUDED from
  the chat/`:model` provider picker (`select_provider`, `ui.rs:3506-3520`, which
  filters to `ProviderType::Llm`) and INCLUDED in `provider login`/`provider
  list`. Rationale: the desired visibility split should be guaranteed by the
  `provider_type`, not by ad-hoc filtering.

### Memory client refactor (forge_repo)

- [ ] Task 8. Convert `ForgeMemoryRepository`
  (`crates/forge_repo/src/memory.rs`) into a stateless gRPC client: remove
  `MEMORY_CONFIG_FILE`, `MemoryConfig`, and `load_config()`; change
  `create_episode`/`retrieve` into inherent methods that accept `server_url: &str`
  and `token: &str` (plus the existing `session_key`/payload) and always perform
  the call (no `Option`/no-op short-circuit here). Drop the
  `impl MemoryRepository for ForgeMemoryRepository`. Rationale: separation of
  concerns — the client only speaks gRPC; configuration resolution moves to the
  layer that owns credentials.
- [ ] Task 9. Make `ForgeRepo<F>`'s `MemoryRepository` impl
  (`crates/forge_repo/src/forge_repo.rs:530-546`) the sole trait implementor and
  the config resolver: in `create_episode`/`retrieve`, look up
  `get_credential(&ProviderId::MNETHOS_MEMORY)` for the bearer token and resolve
  the `mnethos_memory` provider's rendered `url` for `server_url`; when either is
  absent return the no-op result (`Ok(None)` / `Ok(vec![])`), otherwise delegate
  to the gRPC client. Add whatever infra trait bounds the resolution needs
  (matching the bounds already used by `ForgeProviderRepository`). Rationale:
  preserves the "memory is inert when unconfigured" guarantee while sourcing the
  token from the credential store and the URL from provider.json.
- [ ] Task 10. Update doc comments in `crates/forge_repo/src/memory.rs` and
  `crates/forge_domain/src/memory.rs` to describe the credential/provider-based
  config (remove all `memory.json` references). Rationale: keep the
  LLM-facing docs accurate (project convention).

### Tool gating (forge_app)

- [ ] Task 11. Replace the env+file gate in
  `crates/forge_app/src/tool_registry.rs:279-284` with a credential-presence
  check: `memory_supported = <services>.get_credential(&ProviderId::
  MNETHOS_MEMORY).await?.is_some()` (use the existing async services accessor;
  the registry method is already async and performs async lookups). Remove the
  `MNETHOS_MEMORY` env var and `memory.json` existence checks. Rationale: gate on
  the same signal that defines "configured provider", giving one source of truth.

### Backward-compat migration (optional but recommended)

- [ ] Task 12. Add a one-time migration that, on startup, if
  `<base_path>/memory.json` exists and no `mnethos_memory` credential is present,
  writes an `AuthCredential::new_api_key(ProviderId::MNETHOS_MEMORY, <token>)`
  into the credential store (mirroring the existing env-credential migration
  pattern, `migrate_env_credentials`) and then ignores/retires `memory.json`.
  Rationale: existing users with a populated `~/.mnethos/memory.json` keep
  working without manual re-login.

### Testbench alignment (dev harness)

- [ ] Task 13. Update `testbench/setup-memory-user.sh:58-61` to write the AWM
  `awm_` token into the isolated config's `.credentials.json` as a
  `mnethos_memory` api_key credential (instead of writing `memory.json`), and
  point the provider URL at `MEMORY_GRPC_URL`. Rationale: tests must exercise the
  same credential-based path users now use.
- [ ] Task 14. Update `testbench/run-test.sh` so the MEMORY=1 path no longer
  relies on the `MNETHOS_MEMORY` env var (now a no-op) and the MEMORY=0 baseline
  uses a config WITHOUT the `mnethos_memory` credential (true A/B). Update the
  README notes accordingly. Rationale: keep the A/B harness valid under
  credential gating.

### Tests

- [ ] Task 15. Domain unit tests (`provider.rs` test module): `display_name()`
  returns "Mnethos Memory Service"; `FromStr`/round-trip for `mnethos_memory`;
  `built_in_providers()` contains `MNETHOS_MEMORY`; `ProviderType::Memory`
  serde/`Display` == `"memory"`. Use `pretty_assertions`, fixtures, and the
  actual/expected pattern per project guidelines.
- [ ] Task 16. Login-guard test (forge_main): a non-`Llm` configured provider
  skips `finalize_provider_activation` (assert no model/session-config mutation),
  and a memory provider is filtered out of the `select_provider` (`Llm`-only)
  list but present in the login list.
- [ ] Task 17. Repo seam test (forge_repo): with no `mnethos_memory` credential,
  `MemoryRepository::create_episode`/`retrieve` are no-ops (`Ok(None)`/empty);
  with a credential + provider URL present, the gRPC client is invoked with the
  resolved `server_url` + bearer token (use a fake/mocked gRPC infra).
- [ ] Task 18. Gating test (forge_app): `memory_supported` is true iff the
  `mnethos_memory` credential exists.

### Verification & cleanup

- [ ] Task 19. Run `cargo insta test --accept` for the touched crates
  (`forge_domain`, `forge_repo`, `forge_app`, `forge_main`,
  `forge_services`) plus `cargo clippy` on them; update any snapshots that
  legitimately change (e.g. provider listings). NEVER use `--release`.
- [ ] Task 20. Manual smoke (on the Windows box per prior recipe, or locally):
  `mnethos provider login mnethos_memory` → prompts for the key, stores it;
  `mnethos provider list` shows it with host `awm.mnethos.com`; the agent is
  offered `remember`/`mem_search`; `mnethos provider logout mnethos_memory`
  removes it and the tools disappear.

## Verification Criteria

- `mnethos provider login mnethos_memory` shows exactly "Enter your Mnethos
  Memory Service API key", stores a `mnethos_memory` api_key entry in
  `.credentials.json`, and does NOT prompt for or set a chat model.
- `mnethos provider login` (no arg) and `mnethos provider list` include
  "Mnethos Memory Service"; the `:model`/chat provider picker does NOT.
- After login, `mem_search` and `remember` are offered to the agent and reach
  the AWM backend (recall/persist succeed); after `provider logout
  mnethos_memory`, the tools are no longer offered and memory calls are no-ops.
- No code path reads or writes `memory.json` anymore (except the optional
  one-time migration); `rg memory.json` returns only migration + docs/tests.
- `cargo insta test` and `cargo clippy` are green for all touched crates.
- A pre-existing `~/.mnethos/memory.json` is auto-migrated to a credential on
  first run (if Task 12 is included).

## Potential Risks and Mitigations

1. **Exhaustive `match` on `ProviderType` fails to compile after adding
   `Memory`.**
   Mitigation: Task 3 audits all usages; rely on the compiler to flag every
   non-exhaustive site; prefer equality checks over matches where reasonable.
2. **`finalize_provider_activation` runs for memory and corrupts the chat
   session config (wrong default provider/model).**
   Mitigation: Task 6 guards on `provider_type != Llm` before finalize; Task 16
   asserts no session-config mutation for non-LLM providers.
3. **URL rendering at the repo seam is awkward (templates vs `Provider<Url>`).**
   Mitigation: the `mnethos_memory` URL has no `{{params}}`, so it renders to the
   literal; resolve via the provider repository. Alternative fallback: a single
   `const AWM_GRPC_URL` in `memory.rs` used when the provider URL can't be
   rendered at that layer (documented trade-off, one source still preferred).
4. **Gating change breaks the testbench A/B (env var no longer gates).**
   Mitigation: Tasks 13–14 migrate the harness to credential-based config and a
   credential-free MEMORY=0 baseline.
5. **Existing users lose memory after upgrade (memory.json ignored).**
   Mitigation: Task 12 auto-migration; otherwise document a one-time `provider
   login mnethos_memory`.
6. **TLS/endpoint regressions for the gRPC channel.**
   Mitigation: keep `build_channel`'s existing https→webpki-roots logic
   unchanged; production endpoint uses Let's Encrypt (standard roots trusted).

## Alternative Approaches

1. **Reuse `ProviderType::ContextEngine` instead of adding `Memory`.** Less code,
   but conflates two distinct backends and risks the context-engine code paths
   picking up the memory provider. Rejected for clarity/safety.
2. **Keep `server_url` in the credential's `url_params` (declare an optional
   `url_param_vars` with a default).** Makes the endpoint user-configurable but
   adds a prompt line at login, contradicting "just ask for the API key". Offer
   later as an opt-in for self-hosted AWM.
3. **Give `ForgeMemoryRepository` direct access to the credential store (inject
   infra).** Avoids moving resolution to `ForgeRepo`, but couples the gRPC client
   to infra and breaks its dependency-free design. Rejected; the delegation seam
   is cleaner.
4. **Keep `MNETHOS_MEMORY` env as an additional toggle alongside the credential
   gate.** Belt-and-suspenders, but two sources of truth = the hack the requester
   forbids. Rejected in favor of credential-only gating.
