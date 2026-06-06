#!/usr/bin/env bash
# Mnethos memory test harness.
#
# Copies a pristine fixture project into a fresh, ISOLATED git repo and runs the
# mnethos agent on a task INSIDE that copy — the fixture itself is never touched.
#
# ONE memory switch: MEMORY=1 (default) turns memory ON (the single MNETHOS_MEMORY
# flag → the agent is offered the remember + mem_search tools and uses them inside
# its own loop). MEMORY=0 runs a PURE agent with no memory tools at all — the true
# A/B baseline. Either way the agent-task token usage is read back from the
# persisted conversation (DB), so metrics work in both modes.
#
# The memory tools need the isolated test config (run setup-memory-user.sh first →
# testbench/.mnethos-test, a dedicated awm user). Without it, the remember/mem_search
# tools are still offered but their reads/writes are no-ops.
#
# Usage:
#   bash testbench/run-test.sh [TASK_FILE]
#
# Env overrides:
#   MEMORY=0|1          memory flow on/off (default 1)
#   KEEP_MEMORY=1       (MEMORY=1 only) skip the pre-run graph wipe so the run can
#                       RECALL what previous runs stored (default: wipe first)
#   FIXTURE=<dir>       pristine project to copy (default: fixtures/todo-cli)
#   MNETHOS_BIN=<path>  agent binary           (default: target/debug/mnethos)
#   RUN_ROOT=<dir>      where run copies live  (default: $TMPDIR/mnethos-testruns)
#   MNETHOS_EXTRA=...   extra flags passed to mnethos (e.g. "--agent smith -C .")
set -euo pipefail

MEMORY="${MEMORY:-1}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

FIXTURE="${FIXTURE:-$SCRIPT_DIR/fixtures/todo-cli}"
TASK_FILE="${1:-$SCRIPT_DIR/tasks/add-done-command.md}"
MNETHOS_BIN="${MNETHOS_BIN:-$REPO_ROOT/target/debug/mnethos}"
RUN_ROOT="${RUN_ROOT:-${TMPDIR:-/tmp}/mnethos-testruns}"

[ -d "$FIXTURE" ]    || { echo "fixture not found: $FIXTURE" >&2; exit 1; }
[ -f "$TASK_FILE" ]  || { echo "task file not found: $TASK_FILE" >&2; exit 1; }
[ -x "$MNETHOS_BIN" ] || { echo "binary not found/executable: $MNETHOS_BIN  (run: cargo build --bin mnethos)" >&2; exit 1; }

# Read the task NOW, while cwd still resolves a relative TASK_FILE — the agent is
# launched later from inside the copied project dir, where the path would break.
TASK_CONTENT="$(cat "$TASK_FILE")"

stamp="$(date +%Y%m%d-%H%M%S)"
task_name="$(basename "$TASK_FILE" .md)"
RUN_DIR="$RUN_ROOT/${task_name}-${stamp}"

echo ">>> fixture : $FIXTURE"
echo ">>> task    : $TASK_FILE"
echo ">>> binary  : $MNETHOS_BIN"
echo ">>> run dir : $RUN_DIR"
echo

# 1) Isolated copy — the agent never touches the pristine fixture.
# Copy into a subdir named after the fixture so the project's directory
# basename is a meaningful, stable project name (e.g. "todo-cli"), like a real
# repo checkout — consolidation derives the project identity from this.
proj_name="$(basename "$FIXTURE")"
PROJ_DIR="$RUN_DIR/$proj_name"
mkdir -p "$PROJ_DIR"
cp -R "$FIXTURE/." "$PROJ_DIR/"

# 2) Standalone git repo so the agent's git-root is the copy, fully isolated
#    from the mnethos repository.
cd "$PROJ_DIR"
git init -q
git add -A
git -c user.email="test@mnethos.dev" -c user.name="mnethos-test" commit -q -m "fixture baseline"

# 2.5) Decide the memory switch. Always point mnethos at the isolated test
#      config when it exists (own DB + creds + memory provider). MEMORY=1 offers
#      the memory tools (MNETHOS_MEMORY) and wipes the graph first (unless
#      KEEP_MEMORY=1, so a run can recall prior runs). MEMORY=0 is the pure-agent
#      baseline: no memory tools, no recall, no wipe.
TEST_CONFIG_DIR="$SCRIPT_DIR/.mnethos-test"
if [ -f "$TEST_CONFIG_DIR/test-memory.env" ]; then
    # shellcheck disable=SC1090
    source "$TEST_CONFIG_DIR/test-memory.env"
    export MNETHOS_CONFIG="$TEST_CONFIG_DIR"
fi

if [ "$MEMORY" = "1" ]; then
    export MNETHOS_MEMORY=1   # single switch: offers the remember + mem_search tools
    if [ -f "$TEST_CONFIG_DIR/test-memory.env" ]; then
        echo ">>> memory: ON  (full flow; config=$MNETHOS_CONFIG  awm=$AWM_URL)"
        if [ "${KEEP_MEMORY:-0}" = "1" ]; then
            echo ">>> KEEP_MEMORY=1 — NOT wiping; this run can recall existing memory"
        else
            echo ">>> wiping test user's memory graph (DELETE /me/data)..."
            code="$(curl -fsS -o /dev/null -w '%{http_code}' -X DELETE "$AWM_URL/me/data" \
                -H "authorization: Bearer $MNETHOS_TEST_TOKEN" 2>/dev/null || echo ERR)"
            if [ "$code" = "200" ] || [ "$code" = "204" ]; then
                echo ">>> wipe ok ($code) — memory graph is empty"
            else
                echo ">>> WARNING: wipe returned '$code' (expected 200/204) — graph may be non-empty" >&2
            fi
        fi
    else
        echo ">>> memory: ON but NO test config — recall/write are no-ops (run setup-memory-user.sh)"
    fi
else
    echo ">>> memory: OFF  (pure-agent baseline — no memory tools)"
fi
echo

# 3) Run the agent one-shot (timed wall-clock). MNETHOS_MEMORY is exported above
#    only when MEMORY=1; otherwise the agent runs with no memory flow at all.
echo ">>> running agent (memory=$MEMORY)..."
echo "----------------------------------------------------------------"
SECONDS=0
"$MNETHOS_BIN" ${MNETHOS_EXTRA:-} -p "$TASK_CONTENT" \
    || echo "(agent exited non-zero)"
run_secs=$SECONDS
echo "----------------------------------------------------------------"
echo

# 4) Report: what the agent changed.
echo ">>> file changes vs baseline:"
git --no-pager diff --stat || true
echo
echo ">>> new untracked files:"
git --no-pager status --porcelain | grep '^??' || echo "  (none)"
echo
dest="$REPO_ROOT/testbench/runs/${task_name}-${stamp}"
mkdir -p "$dest"
git --no-pager diff > "$dest/model.diff" 2>/dev/null || true

# 5) METRICS — wall-clock + token accounting. The agent's own requests (including
#    any remember/mem_search tool calls, which run inside the agent loop) are read
#    from the PERSISTED conversation (DB), so this works whether or not memory was
#    on. NOTE: tolerate a missing DB without tripping `set -euo pipefail`.
db="${MNETHOS_CONFIG:-/nonexistent}/.mnethos.db"
dbtok() {
    [ -f "$db" ] || { echo 0; return; }
    sqlite3 "$db" "select context from conversations order by updated_at desc limit 1;" 2>/dev/null \
        | jq "[.messages[].usage // empty | (.$1.actual // .$1)] | add // 0" 2>/dev/null || echo 0
}
task_total="$(dbtok total_tokens)"
task_prompt="$(dbtok prompt_tokens)"
task_completion="$(dbtok completion_tokens)"
task_cached="$(dbtok cached_tokens)"
files_changed="$(git --no-pager diff --numstat 2>/dev/null | wc -l | tr -d ' ')"

jq -n --arg task "$task_name" --argjson mem "$MEMORY" --argjson wall "$run_secs" \
   --argjson files "${files_changed:-0}" \
   --argjson tt "${task_total:-0}" --argjson tp "${task_prompt:-0}" \
   --argjson tc "${task_completion:-0}" --argjson tk "${task_cached:-0}" \
   '{task:$task, memory:$mem, wall_clock_secs:$wall, files_changed:$files,
     tokens:{agent_task:{total:$tt, prompt:$tp, completion:$tc, cached:$tk}}}' \
   > "$dest/metrics.json" 2>/dev/null

echo ">>> METRICS  (memory=$MEMORY)"
echo "      wall_clock        : ${run_secs}s"
echo "      agent_task tokens : total=$task_total  prompt=$task_prompt  completion=$task_completion  cached=$task_cached"
echo "      files_chg         : $files_changed"
echo
echo ">>> mirrored for inspection: $dest"
echo ">>> inspect: cat \"$dest/metrics.json\"    |    model diff: $dest/model.diff"
echo ">>> metrics: $dest/metrics.json"
