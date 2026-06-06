#!/usr/bin/env bash
# One-time setup for memory end-to-end tests. Run when awm (ai-working-memory) is
# UP. Creates a DEDICATED awm test user + API key and builds an isolated mnethos
# test config (testbench/.mnethos-test) wired to the memory provider as that
# user, so test runs never touch your real ~/.mnethos or your real memory graph.
#
# Idempotent: re-running re-uses the same user (register is tolerated to fail if
# it already exists) and refreshes the API key + config.
#
# Env overrides:
#   AWM_URL=http://localhost:8083          awm REST base URL
#   MEMORY_GRPC_URL=http://localhost:8084  awm gRPC (memory provider) URL
#   TEST_EMAIL / TEST_PASSWORD             test user credentials
set -euo pipefail

AWM_URL="${AWM_URL:-http://localhost:8083}"
MEMORY_GRPC_URL="${MEMORY_GRPC_URL:-http://localhost:8084}"
TEST_EMAIL="${TEST_EMAIL:-mnethos-testbench@example.com}"
TEST_PASSWORD="${TEST_PASSWORD:-mnethos-testbench-pw-change-me}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CONFIG_DIR="$SCRIPT_DIR/.mnethos-test"

echo ">>> awm REST: $AWM_URL   memory gRPC: $MEMORY_GRPC_URL"
echo ">>> test user: $TEST_EMAIL"

# 1) Register the test user (tolerate 'already exists').
curl -fsS -X POST "$AWM_URL/auth/register" \
  -H 'content-type: application/json' \
  -d "{\"email\":\"$TEST_EMAIL\",\"password\":\"$TEST_PASSWORD\"}" >/dev/null 2>&1 \
  && echo ">>> registered new user" || echo ">>> user already exists (ok)"

# 2) Login → access token.
access="$(curl -fsS -X POST "$AWM_URL/auth/login" \
  -H 'content-type: application/json' \
  -d "{\"email\":\"$TEST_EMAIL\",\"password\":\"$TEST_PASSWORD\"}" | jq -r '.tokens.accessToken')"
[ -n "$access" ] && [ "$access" != "null" ] || { echo "ERROR: login failed"; exit 1; }

# 3) Create a long-lived API key (awm_ token) — plaintext returned once.
token="$(curl -fsS -X POST "$AWM_URL/me/api-keys" \
  -H "authorization: Bearer $access" -H 'content-type: application/json' \
  -d '{"name":"mnethos-testbench"}' | jq -r '.plaintextKey')"
[ -n "$token" ] && [ "$token" != "null" ] || { echo "ERROR: api key creation failed"; exit 1; }

# 4) Build the isolated mnethos test config dir: carry over the LLM credential +
#    model selection from the real config, then add the memory backend as a
#    `mnethos_memory` provider credential (the same credential-based path users
#    now use — no bespoke memory.json), and override the memory provider URL to
#    the local AWM gRPC endpoint.
rm -rf "$CONFIG_DIR"; mkdir -p "$CONFIG_DIR"
if [ -f "$HOME/.mnethos/.credentials.json" ]; then
  cp "$HOME/.mnethos/.credentials.json" "$CONFIG_DIR/.credentials.json"
else
  echo '[]' > "$CONFIG_DIR/.credentials.json"
fi
[ -f "$HOME/.mnethos/.config.json" ] && cp "$HOME/.mnethos/.config.json" "$CONFIG_DIR/.config.json" || true
[ -f "$HOME/.mnethos/.mnethos.toml" ] && cp "$HOME/.mnethos/.mnethos.toml" "$CONFIG_DIR/.mnethos.toml" || true

# Append the mnethos_memory api_key credential (drop any stale one first), so the
# memory tools (remember + mem_search) are gated ON for this run.
tmp_creds="$(mktemp)"
jq --arg t "$token" \
  '[.[] | select(.id != "mnethos_memory")] + [{id:"mnethos_memory", auth_details:{api_key:$t}}]' \
  "$CONFIG_DIR/.credentials.json" > "$tmp_creds"
mv "$tmp_creds" "$CONFIG_DIR/.credentials.json"
chmod 600 "$CONFIG_DIR/.credentials.json"

# Stash a credentials variant WITHOUT the memory credential for the MEMORY=0
# baseline (true A/B: memory tools gated OFF).
jq '[.[] | select(.id != "mnethos_memory")]' \
  "$CONFIG_DIR/.credentials.json" > "$CONFIG_DIR/.credentials.nomem.json"
chmod 600 "$CONFIG_DIR/.credentials.nomem.json"

# Override the mnethos_memory provider URL to the local AWM gRPC endpoint
# (the built-in default points at production awm.mnethos.com).
jq -n --arg url "$MEMORY_GRPC_URL" \
  '[{id:"mnethos_memory", provider_type:"memory", url_param_vars:[], url:$url, auth_methods:["api_key"]}]' \
  > "$CONFIG_DIR/provider.json"

# 5) Persist token + URLs for the runner's pre-run wipe (DELETE /me/data).
cat > "$CONFIG_DIR/test-memory.env" <<EOF
AWM_URL=$AWM_URL
MEMORY_GRPC_URL=$MEMORY_GRPC_URL
MNETHOS_TEST_TOKEN=$token
EOF

echo ">>> ready. test config: $CONFIG_DIR"
echo ">>> run-test.sh will now use MNETHOS_CONFIG=$CONFIG_DIR and wipe this user before each run."
