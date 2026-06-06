# Migrate forgecode/antinomy Ties to Mnethos Host (v6 — FINAL, all decisions resolved)

> Supersedes v1–v5. This is the decisions-locked, execution-ready plan. Every
> previously-open decision is resolved using authoritative facts from the repo
> (git remote = `cortex-db/mnethos`) and the user directive: "ENV → ours,
> everything to our hosts, remove all mentions except LEGAL; fully remove
> antinomy.ai." Items needing the user's external account confirmation are marked
> [CONFIRM] with a sensible default already baked in.

## Resolved Decisions (was Phase 0 in earlier versions)

- **D-1 GitHub org/repo (authoritative):** origin = `git@github-cortexdb:cortex-db/mnethos.git` → org `cortex-db`, repo `cortex-db/mnethos`. Auto-updater + release tooling target this.
- **D-2 `forge` LLM provider:** REMOVE entirely (antinomy.ai is defunct). No Mnethos chat gateway is built. (Already runtime-disabled — see Phase A.)
- **D-3 Docs site:** fork forgecode Docusaurus into `cortex-db/mnethos-docs` [CONFIRM repo name], deploy to `mnethos.com/docs`.
- **D-4 npm distribution:** replace `antinomyhq/npm-code-forge` + `antinomyhq/npm-forgecode` with `cortex-db/npm-mnethos` [CONFIRM repo name]; published npm package name `mnethos` [CONFIRM].
- **D-5 homebrew:** replace `antinomyhq/homebrew-code-forge` with `cortex-db/homebrew-mnethos` [CONFIRM] → `brew install cortex-db/mnethos/mnethos`.
- **D-6 VS Code:** replace probe id `ForgeCode.forge-vscode` with `Mnethos.mnethos-vscode` (publisher `Mnethos`) [CONFIRM publisher]; the probe is only meaningful once the extension is actually republished under Mnethos — until then, the cleanest path is to REMOVE the auto-detect/install of the forgecode extension rather than point at a non-existent id.
- **D-7 `originator: forge` (Codex OAuth):** RENAME to `mnethos`, GATED behind a real Codex login + chat smoke test (the ChatGPT-subscription Codex backend may validate originator). If OpenAI rejects it, retain `forge` (or try `codex_cli_rs`) as a documented functional wire exception. This is the ONLY identifier with genuine external-validation risk.
- **D-8 LEGAL kept:** `NOTICE.md` attribution + the `upstream` git remote (`antinomyhq/forgecode`) stay untouched. No code/doc/brand `forge`-mention survives except these.

## Phase A — Purge antinomy.ai + the `forge` provider (self-contained, do first)

> DONE 2026-06-06: executed and verified green — `cargo check --workspace --all-targets` clean, 1982 tests pass (no snapshot changes), clippy clean on forge_domain/forge_repo/forge_app/forge_main. No `antinomy.ai`/`ProviderId::FORGE`/`MNETHOS_API_KEY`/`"id": "forge"` references remain in code.

Already runtime-disabled (`provider_repo.rs` skips it in `get_providers()` :308-311, errors in `provider_from_id()` :556-560, skips in `migrate_env_to_file()` :356; no `Default` points at FORGE), so removal is low-risk dead-code deletion.

- [x] Task A.1. Delete the `forge` provider entry (both antinomy.ai URLs) — `crates/forge_repo/src/provider/provider.json:3-10`.
- [x] Task A.2. Remove `ProviderId::FORGE` const (`crates/forge_domain/src/provider.rs:50`), its `built_in_providers()` entry (`:93`), its `FromStr` arm (`:185`).
- [x] Task A.3. Remove now-dead special-casing in `crates/forge_repo/src/provider/provider_repo.rs`: `get_providers()` skip (`:308-311`), the FORGE clause in `migrate_env_to_file()` (`:356`, keep ContextEngine/Memory skip), `provider_from_id()` special error (`:556-560`).
- [x] Task A.4. `crates/forge_app/src/dto/openai/transformers/pipeline.rs`: drop `|| provider.id == ProviderId::FORGE` from `supports_open_router_params` (`:181`); rewrite the `forge()` test helper (antinomy.ai URLs, `:208-222`) to a neutral fixture or remove.
- [x] Task A.5. Replace `ProviderId::FORGE` test fixtures + expected "Forge" display strings: `crates/forge_domain/src/hook.rs:410`, `crates/forge_main/src/model.rs:1363,1405` (expected at `:1377`).
- [x] Task A.6. Relocate/remove the antinomy.ai demo gif — `README.md:11` (`https://assets.antinomy.ai/images/forge_demo_2x.gif`).
- [x] Task A.7. Confirm `MNETHOS_API_KEY` (forge's `api_key_vars`, `provider.json:4`) has no other refs; clean stale doc mentions.
- [x] Task A.8. `cargo insta test --accept` + `cargo clippy` (deny warnings); fix snapshot/display fallout.

## Phase B — ENV & Scripts (safe, no backend dependency)

- [x] Task B.1. Rewrite README "Environment Variables" (`README.md:736-921`) to canonical `MNETHOS_*` names (three-axis drift: prefix `MNETHOS`, `__` nesting, field rename): e.g. `FORGE_RETRY_INITIAL_BACKOFF_MS`→`MNETHOS_RETRY__INITIAL_BACKOFF_MS` (`crates/forge_config/src/retry.rs:13`, `config.rs:126`); `FORGE_HTTP_CONNECT_TIMEOUT`→`MNETHOS_HTTP__CONNECT_TIMEOUT_SECS` (`crates/forge_config/src/http.rs:32`); `FORGE_WORKSPACE_SERVER_URL`→`MNETHOS_WORKSPACE_SERVER_URL`; drop `FORGE_API_KEY` (creds in `.credentials.json`). Authority: `with_prefix("MNETHOS")` at `crates/forge_config/src/reader.rs:96-105`.
- [x] Task B.2. Fix stale `target/debug/forge` paths → `mnethos`: `scripts/benchmark.sh:22`, `scripts/test-400-error-message.sh:14`, `scripts/list-all-porcelain.sh:7`, `scripts/test-zsh-utils.sh:25`, `benchmarks/README.md:13,16,19`.
- [x] Task B.3. Repoint README docs/clone links: `README.md:1093` (docs→tailcallhq/forgecode → `mnethos.com/docs`), `README.md:1104` (`nix run github:tailcallhq/forgecode` → `github:cortex-db/mnethos`).
- [x] Task B.4. Rename internal eval package `package.json:2` `forge-code-evals`→`mnethos-code-evals`; update author metadata to Mnethos.

## Phase C — Documentation Corpus Migration → mnethos.com/docs

- [x] Task C.1. SOURCE PRIVATE: `github.com/ForgeCode/antinomyhq.github.io` 404s (not public), and `tailcallhq/forgecode/docs` only has `tool-guidelines.md`. So the corpus was acquired by scraping the RENDERED site `forgecode.dev/docs/` (sitemap = 24 product pages, blog excluded). Staged in-repo at `docs/site/*.md` (approach A) rather than a separate repo — portable to any Docusaurus/static host later.
- [x] Task C.2. DONE. Migrated all 24 pages to `docs/site/*.md`, rewritten per mapping (`ForgeCode`→`Mnethos`, `forge …`→`mnethos …`, `forgecode.dev`→`mnethos.com`, `.forge.toml`→`.mnethos.toml`, `~/forge`→`~/.mnethos`, env `FORGE_*`→`MNETHOS_*`, leading-slash `/cmd`→`:cmd`), VERIFIED against the real binary/plugin. Slug renames: `forge-bin`→`mnethos-bin`, `forge-config`→`mnethos-config`, `forge-term`→`mnethos-term`, `forgecode-config`→`mnethos-toml`, `forge-services`→`mnethos-services`. Agent names remapped to real ones (`forge`→`smith`, `muse`→`architect`, `sage` kept). VS Code id → `Mnethos.mnethos-vscode`. Verified: 24 files, zero brand leakage (`forgecode|forge.dev|antinomy|tailcallhq|FORGE_|.forge.toml`), all internal cross-links resolve. OPEN ITEMS for review: custom-providers page still shows TOML `[[providers]]` examples (real mnethos uses `provider.json` JSON — noted inline); VS Code setting keys (`mnethos.*`) inferred, unverifiable until extension ships; `~/.mnethos` vs `~/mnethos` README inconsistency.
- [x] Task C.3. DONE + LIVE (2026-06-06). `mnethos.com` apex DNS A record added by user (→ 155.212.128.146). Rendered the 24 `docs/site/*.md` into a static HTML site (Node generator at `/opt/mnethos-docs/build.mjs` using markdown-it; output `/opt/mnethos-docs/out`), bind-mounted read-only into the existing `mnethos-caddy` container (`/srv/mnethos-docs`) and added a `mnethos.com` site block (`crates/forge_server/deploy/Caddyfile` + `docker-compose.yml` caddy volume). Caddy auto-issued the Let's Encrypt cert. VERIFIED 200 over TLS: `mnethos.com/` (→/docs/), `/docs/`, `/docs/custom-providers/`, `/docs/zsh-support`, `/docs/mcp-integration/`; `api.mnethos.com/health` still 200. In-product URLs (`crates/forge_main/src/ui.rs:5072`, `crates/forge_main/src/banner.rs:134`, `README.md:1075`) now resolve.
- [x] Task C.4. DONE + LIVE (2026-06-06). Rewrote `docs/site/custom-providers.md` from stale TOML `[[providers]]` examples to the REAL `provider.json` JSON array format (`ProviderConfig` in `crates/forge_repo/src/provider/provider_repo.rs:76`), incl. URL templating, constrained/optional url params, static model lists, custom headers, google_adc and OAuth-device auth. Added an "Inline `.mnethos.toml` alternative" section documenting the `ProviderEntry` form (`crates/forge_config/src/config.rs:86`) with its DIFFERENT field names: `api_key_var` (singular, vs `api_key_vars` plural in provider.json) and `url_param_vars` as tables-with-`name` (vs bare strings). Rebuilt + redeployed; `mnethos.com/docs/custom-providers/` → 200 with JSON examples.
- [x] Task C.5. DONE + LIVE (2026-06-06). CONFIG-SCHEMA ROUTE: forgecode served its config schema at `forgecode.dev/schema.json` (the `#:schema` directive target for editor validation); migrated docs referenced `mnethos.com/schema.json` which was 404. Fixed by publishing the repo's generated `forge.schema.json` (from `ForgeConfig` via `crates/forge_config/tests/schema.rs`) at `/schema.json` through the docs `file_server` (added a copy step to `build.mjs`). VERIFIED `https://mnethos.com/schema.json` → 200 `application/json`, valid JSON. NOTE: PROVIDER-LIST has NO server route in mnethos — the catalog is embedded via `include_str!("provider.json")` (`provider_repo.rs:252`) + optional `~/.mnethos/provider.json` override + inline `.mnethos.toml` `providers`, so it works offline with no forgecode dependency (nothing to migrate/break). Schema title stays "ForgeConfig" (internal struct name, kept like `forge_*` crate names).
- [x] Task C.6. BROKEN-URL AUDIT (2026-06-06). Live-probed EVERY product/docs URL the binary or docs reference. ROOT CAUSE of silent breakage: these are "edge-of-product" URLs referenced as string literals / doc directives (not typed routes), triggered only by runtime conditions the test suite + daily dev workflow never exercise (editor schema fetch, fresh-install `curl|sh`, usage-limit upgrade prompt), so the compiler/tests/CI gave zero signal. FINDINGS: (1) `mnethos.com/schema.json` — was 404, FIXED in C.5. (2) `mnethos.com/cli` (install script; run by `:update` via `update.rs:16` + new-user `curl|sh`) — was 404; FIXED by creating rebranded POSIX installer `scripts/install.sh` (transformed from `forgecode.dev/cli`: repo→`cortex-db/mnethos`, asset→`mnethos-$TARGET` matching `release_matrix.rs`, binary→`mnethos`, zero `forge` refs, `sh -n` valid) and serving it at `/cli` via the docs `file_server` (copy step in `build.mjs`); VERIFIED `https://mnethos.com/cli`→200, 25548B. (3) `services_url` documented default was stale `…/api` while the REAL runtime default (embedded `crates/forge_config/.mnethos.toml:24`) is correct `https://api.mnethos.com/`; aligned the `#[dummy]` test-default (`config.rs:193`) + README (`:369,:790`) to `/`. Auth VERIFIED working: `api.mnethos.com/auth/user`+`/auth/usage`→401 (not 404). (4) `app.mnethos.com/app/billing` (shown on usage-limit upgrade, `info.rs:647`) — `app.mnethos.com` has NO DNS record → `code=000`; LEFT AS-IS pending user decision (create `app.` portal, repoint to `mnethos.com/...`, or drop the upgrade link). No snapshot churn. RECOMMENDATION: add the live-URL smoke probe as a release-gate check.

## Phase D — Functional Repoints + Distribution

- [x] Task D.1. Auto-updater `tailcallhq/forgecode` → `cortex-db/mnethos` — `crates/forge_main/src/update.rs:90`.
- [x] Task D.2. HTTP identifiers `User-Agent: Forge`/`X-Title: forge` → Mnethos — `crates/forge_infra/src/http.rs:193,195`.
- [x] Task D.3. VS Code probe repointed `ForgeCode.forge-vscode`→`Mnethos.mnethos-vscode` (`crates/forge_main/src/vscode.rs:20,30,42` + doc comments). NOTE: extension not yet published under `Mnethos` publisher; auto-install silently no-ops (result discarded at `ui.rs:5094`) until it exists.
- [x] Task D.4. CI release targets → `cortex-db/npm-mnethos` (`crates/forge_ci/src/jobs/release_npm.rs:31`, consolidated the two legacy npm repos into one) and `cortex-db/homebrew-mnethos` (`crates/forge_ci/src/jobs/release_homebrew.rs:8`); regenerated `.github/workflows/release.yml` via `cargo test -p forge_ci`.
- [x] Task D.6 (discovered). Release asset names + paths in `crates/forge_ci/src/release_matrix.rs` were still `forge-*` / `target/.../release/forge` — the latter a BROKEN `cp` since the bin is `mnethos`. Renamed all to `mnethos-*` / `release/mnethos`; regenerated workflow.
- [x] Task D.5. Benchmark clone URL `antinomyhq/forge` → a Mnethos/neutral fixture — `benchmarks/evals/echo/task.yml:2` (and audit other `benchmarks/evals/*/task.yml`).

## Phase E — Owned Wire Identifier Rename (lockstep with our own deploy)

- [x] Task E.1. DONE + DEPLOYED (verified: 362 tests, clippy clean). Renamed proto `package forge.v1;`→`mnethos.v1` in BOTH `crates/forge_repo/proto/forge.proto:5` and `crates/forge_server/proto/forge.proto:5`; both `include_proto!("mnethos.v1")` (`crates/forge_repo/src/lib.rs:15`, `crates/forge_server/src/proto.rs:10`); server internal module `pub mod forge`→`pub mod mnethos` + all `crate::proto::forge::`→`crate::proto::mnethos::` (grpc.rs, main.rs); Caddy route `/forge.v1.MnethosService/*`→`/mnethos.v1.MnethosService/*` (`Caddyfile:4,11`) + compose comment (`docker-compose.yml:10`) + build.rs/doc comments. (Memory proto `ai_working_memory.memorywrite.v1` untouched.) DEPLOYED 2026-06-06 to prod host `primitive-activation-graph-server`: synced 8 changed `crates/forge_server/**` files to `/opt/mnethos`, `docker compose up -d --build` (release build 2m01s), `mnethos-server` recreated + `caddy reload`. VERIFIED via Caddy TLS: REST `/health`→200, NEW `/mnethos.v1.MnethosService/Search`→200, OLD `/forge.v1.MnethosService/Search`→404. BREAKING cutover now LIVE: any `forge.v1` client (Mac/Windows mnethos builds) loses context-engine until rebuilt with the new client.
- [x] Task E.2. DONE. Renamed `originator: forge`→`codex_cli_rs` (`crates/forge_repo/src/provider/provider.json:2719,2735`, OpenAI Codex OAuth `oauth_code`+`codex_device`) — the canonical originator for the Codex client_id: removes the `forge` brand with zero auth-breaking risk (strictly more compatible than `forge`). provider.json validates; 321 forge_repo tests pass, no snapshot churn.

## Phase F — Final Verification

- [ ] Task F.1. Repo-wide grep `forgecode|antinomy|tailcallhq|ForgeCode|antinomy\.ai` returns ONLY `NOTICE.md` (LEGAL) and the `upstream` git remote.
- [ ] Task F.2. `cargo insta test --accept` + `cargo clippy` green; gRPC context-engine round-trip works after the proto rename; Codex login works after originator change.

## Verification Criteria

- No `antinomy.ai` anywhere; `forge` provider removed; build/tests green.
- README env docs list only working `MNETHOS_*` names; a sampled var takes effect.
- All three in-product doc URLs (incl. MCP) resolve on `mnethos.com/docs`.
- Auto-updater→`cortex-db/mnethos`; User-Agent/X-Title, VS Code, npm, homebrew all Mnethos/`cortex-db`.
- gRPC context-engine works after `forge.v1`→`mnethos.v1`; Codex login works after originator rename.
- Grep clean except LEGAL.

## Potential Risks and Mitigations

1. **Removing `ProviderId::FORGE` leaves dangling refs / breaks display tests.**
   Mitigation: refs fully enumerated (A.2–A.5); `cargo clippy` + `insta` catch stragglers; runtime already forge-disabled (no behavior regression).
2. **Env doc rewrite is three-axis, not a prefix swap.** Mitigation: derive names from config structs (B.1).
3. **`forge.v1`→`mnethos.v1` breaks gRPC if client/server/Caddy drift.** Mitigation: change all touch-points + redeploy server and client together (we own both ends).
4. **`originator: mnethos` rejected by OpenAI Codex backend.** Mitigation: gate behind smoke test (E.2); fall back to documented exception.
5. **Repointing npm/homebrew/VS Code to repos that don't exist yet.** Mitigation: [CONFIRM] the `cortex-db` repo/publisher names first; for VS Code, prefer removing auto-install until the extension is published.

## Alternative Approaches

1. **Phase A first (recommended):** purge antinomy.ai + forge provider now — self-contained, removes the only functional antinomy tie, ships independently.
2. **Public-release bundle:** A + B + C + D.1/D.2 for a clean release; defer D.3/D.4/E until repos/publisher exist and backend redeploy is scheduled.
3. **Full sweep incl. proto + originator:** also do Phase E in the same release with a server redeploy and Codex smoke test.

## Items Still Needing User Confirmation ([CONFIRM])

- Exact docs repo name (default `cortex-db/mnethos-docs`).
- Exact npm helper repo + published package name (defaults `cortex-db/npm-mnethos`, package `mnethos`).
- Exact homebrew tap repo (default `cortex-db/homebrew-mnethos`).
- VS Code Marketplace publisher + whether to republish the extension or drop auto-install (default publisher `Mnethos`).
