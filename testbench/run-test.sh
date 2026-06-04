#!/usr/bin/env bash
# Mnethos memory test harness.
#
# Copies a pristine fixture project into a fresh, ISOLATED git repo and runs the
# mnethos agent on a task INSIDE that copy — the fixture itself is never touched.
# The consolidation observer (MNETHOS_MEMORY_OBSERVE) writes a snapshot we can
# inspect afterwards to see exactly what data a real memory step would receive.
#
# Usage:
#   bash testbench/run-test.sh [TASK_FILE]
#
# Env overrides:
#   FIXTURE=<dir>       pristine project to copy (default: fixtures/todo-cli)
#   MNETHOS_BIN=<path>  agent binary           (default: target/debug/mnethos)
#   RUN_ROOT=<dir>      where run copies live  (default: $TMPDIR/mnethos-testruns)
#   MNETHOS_EXTRA=...   extra flags passed to mnethos (e.g. "--agent forge -C .")
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

FIXTURE="${FIXTURE:-$SCRIPT_DIR/fixtures/todo-cli}"
TASK_FILE="${1:-$SCRIPT_DIR/tasks/add-done-command.md}"
MNETHOS_BIN="${MNETHOS_BIN:-$REPO_ROOT/target/debug/mnethos}"
RUN_ROOT="${RUN_ROOT:-${TMPDIR:-/tmp}/mnethos-testruns}"

[ -d "$FIXTURE" ]    || { echo "fixture not found: $FIXTURE" >&2; exit 1; }
[ -f "$TASK_FILE" ]  || { echo "task file not found: $TASK_FILE" >&2; exit 1; }
[ -x "$MNETHOS_BIN" ] || { echo "binary not found/executable: $MNETHOS_BIN  (run: cargo build --bin mnethos)" >&2; exit 1; }

stamp="$(date +%Y%m%d-%H%M%S)"
task_name="$(basename "$TASK_FILE" .md)"
RUN_DIR="$RUN_ROOT/${task_name}-${stamp}"

echo ">>> fixture : $FIXTURE"
echo ">>> task    : $TASK_FILE"
echo ">>> binary  : $MNETHOS_BIN"
echo ">>> run dir : $RUN_DIR"
echo

# 1) Isolated copy — the agent never touches the pristine fixture.
mkdir -p "$RUN_DIR"
cp -R "$FIXTURE/." "$RUN_DIR/"

# 2) Standalone git repo so the agent's git-root is the copy, fully isolated
#    from the mnethos repository.
cd "$RUN_DIR"
git init -q
git add -A
git -c user.email="test@mnethos.dev" -c user.name="mnethos-test" commit -q -m "fixture baseline"

# 3) Run the agent one-shot with memory observation enabled.
echo ">>> running agent (memory observation ON)..."
echo "----------------------------------------------------------------"
MNETHOS_MEMORY_OBSERVE=1 "$MNETHOS_BIN" ${MNETHOS_EXTRA:-} -p "$(cat "$TASK_FILE")" \
    || echo "(agent exited non-zero)"
echo "----------------------------------------------------------------"
echo

# 4) Report: what changed + where the consolidation snapshot landed.
echo ">>> file changes vs baseline:"
git --no-pager diff --stat || true
echo
echo ">>> new untracked files:"
git --no-pager status --porcelain | grep '^??' || echo "  (none)"
echo
dest="$REPO_ROOT/testbench/runs/${task_name}-${stamp}"
mkdir -p "$dest"
git --no-pager diff > "$dest/model.diff" 2>/dev/null || true

echo ">>> RETRIEVAL (pre-request) snapshot(s):"
if ls .mnethos/retrieval/*.json >/dev/null 2>&1; then
    ls -la .mnethos/retrieval/*.json
    for f in .mnethos/retrieval/*.json; do cp "$f" "$dest/retrieval-$(basename "$f")"; done
    echo "  (recall not implemented yet — 'recalled' will be empty)"
else
    echo "  (none written)"
fi
echo
echo ">>> CONSOLIDATION (post-task) snapshot(s):"
if ls .mnethos/consolidation/*.json >/dev/null 2>&1; then
    ls -la .mnethos/consolidation/*.json
    for f in .mnethos/consolidation/*.json; do cp "$f" "$dest/consolidation-$(basename "$f")"; done
else
    echo "  (none written — check that the agent reached the End event)"
fi
echo
echo ">>> mirrored for inspection: $dest"
echo ">>> inspect: jq . \"$dest\"/*.json | less    |    model diff: $dest/model.diff"
