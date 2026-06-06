#!/usr/bin/env bash
# A/B batch with health-gating + medians.
#
# Waits for a HEALTHY model-API window (a trivial timed prompt must finish fast),
# then runs N samples of each branch for the delete task:
#   * baseline : MEMORY=0  — pure agent, no memory flow at all
#   * warm     : MEMORY=1  — re-seed (add-done, wipes) then recall (add-delete, KEEP)
# Every run is bounded by a portable bash timeout so a hung API request can't
# stall the batch; timed-out / non-completing runs are dropped. Emits per-run
# metrics + medians.
#
# Env:
#   N=3                 samples per branch
#   RUN_TIMEOUT=420     hard cap (s) per mnethos run
#   PROBE_TIMEOUT=75    cap (s) for the health probe
#   HEALTH_INTERVAL=120 seconds between health probes
#   HEALTH_MAX_WAIT=7200 give up waiting for a healthy window after this many s
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
BIN="$REPO_ROOT/target/debug/mnethos"
TESTCFG="$SCRIPT_DIR/.mnethos-test"
TASK_DELETE="$SCRIPT_DIR/tasks/add-delete-command.md"
TASK_DONE="$SCRIPT_DIR/tasks/add-done-command.md"

N="${N:-3}"
RUN_TIMEOUT="${RUN_TIMEOUT:-420}"
PROBE_TIMEOUT="${PROBE_TIMEOUT:-75}"
HEALTH_INTERVAL="${HEALTH_INTERVAL:-120}"
HEALTH_MAX_WAIT="${HEALTH_MAX_WAIT:-7200}"

OUT="$SCRIPT_DIR/runs/ab-batch-$(date +%Y%m%d-%H%M%S)"
mkdir -p "$OUT"
log() { echo "[$(date +%H:%M:%S)] $*"; }
cleanup_stray() { pkill -f "$BIN -p" 2>/dev/null || true; }
newest_metrics() { ls -t "$SCRIPT_DIR/runs/$1"-*/metrics.json 2>/dev/null | head -1; }

# Portable timeout (macOS has no `timeout`): run cmd in background, a watchdog
# TERMs it + pkills any hung mnethos after $1 seconds, then KILLs. Returns cmd rc
# (non-zero if it was killed). stdout/stderr of the caller carry through.
run_with_timeout() { # $1=secs; rest=command...
    local secs="$1"; shift
    "$@" &
    local pid=$!
    ( sleep "$secs"; kill -TERM "$pid" 2>/dev/null; pkill -f "$BIN -p" 2>/dev/null
      sleep 3; kill -KILL "$pid" 2>/dev/null ) &
    local wpid=$!
    wait "$pid" 2>/dev/null; local rc=$?
    kill "$wpid" 2>/dev/null; wait "$wpid" 2>/dev/null
    return $rc
}

guarded_run() { # args: TASK_FILE LOGFILE  (MEMORY/KEEP_MEMORY exported by caller)
    local task="$1" logf="$2"
    run_with_timeout "$RUN_TIMEOUT" bash "$SCRIPT_DIR/run-test.sh" "$task" >"$logf" 2>&1
    local rc=$?
    cleanup_stray
    return $rc
}

# Health probe: a trivial prompt in a throwaway cwd must finish < PROBE_TIMEOUT.
# Runs BEFORE the batch so its conversation is older than the measured runs.
healthy() {
    local d; d="$(mktemp -d)"
    ( cd "$d" && MNETHOS_CONFIG="$TESTCFG" run_with_timeout "$PROBE_TIMEOUT" \
        "$BIN" -p "Reply with exactly: OK" >/dev/null 2>&1 )
    local rc=$?
    rm -rf "$d"; cleanup_stray
    return $rc
}

[ -x "$BIN" ] || { log "binary missing: $BIN"; exit 1; }
[ -f "$TESTCFG/test-memory.env" ] || { log "no test config ($TESTCFG) — run setup-memory-user.sh"; exit 1; }

log "waiting for a healthy API window (probe<=${PROBE_TIMEOUT}s, every ${HEALTH_INTERVAL}s)..."
waited=0
until healthy; do
    log "API not healthy yet; retry in ${HEALTH_INTERVAL}s (waited ${waited}s)"
    sleep "$HEALTH_INTERVAL"; waited=$((waited + HEALTH_INTERVAL))
    if [ "$waited" -ge "$HEALTH_MAX_WAIT" ]; then
        log "gave up waiting for healthy window after ${waited}s"; exit 2
    fi
done
log "API healthy — starting batch (N=$N, out=$OUT)"

for i in $(seq 1 "$N"); do
    log "round $i/$N — baseline (MEMORY=0)"
    export MEMORY=0; unset KEEP_MEMORY 2>/dev/null || true
    if guarded_run "$TASK_DELETE" "$OUT/baseline-$i.log"; then
        m="$(newest_metrics add-delete-command)"
        [ -n "$m" ] && cp "$m" "$OUT/baseline-$i.json" && log "  baseline-$i tokens=$(jq -r '.tokens.agent_task.total' "$m") files=$(jq -r '.files_changed' "$m")"
    else
        log "  baseline-$i FAILED (timeout/err) — dropped"
    fi

    log "round $i/$N — seed (MEMORY=1, wipe + add-done)"
    export MEMORY=1; unset KEEP_MEMORY 2>/dev/null || true
    guarded_run "$TASK_DONE" "$OUT/seed-$i.log" || log "  seed-$i failed (continuing)"

    log "round $i/$N — warm (MEMORY=1 KEEP, recall + add-delete)"
    export MEMORY=1 KEEP_MEMORY=1
    if guarded_run "$TASK_DELETE" "$OUT/warm-$i.log"; then
        m="$(newest_metrics add-delete-command)"
        [ -n "$m" ] && cp "$m" "$OUT/warm-$i.json" && log "  warm-$i tokens=$(jq -r '.tokens.agent_task.total' "$m") files=$(jq -r '.files_changed' "$m")"
    else
        log "  warm-$i FAILED (timeout/err) — dropped"
    fi
done

# --- medians (only over runs that actually completed: files_changed>=2) ---
summarize() { # $1 = baseline|warm
    local files; files=$(ls "$OUT/$1"-*.json 2>/dev/null || true)
    [ -z "$files" ] && { echo "{\"branch\":\"$1\",\"n_ok\":0,\"n_total\":0}"; return; }
    # shellcheck disable=SC2086
    jq -s --arg b "$1" '
        def med(f): (map(f)|sort) as $s | ($s|length) as $n
                    | if $n==0 then null else $s[(($n-1)/2)|floor] end;
        map(select(.files_changed>=2)) as $ok |
        {branch:$b, n_ok:($ok|length), n_total:length,
         agent_task_median: ($ok|med(.tokens.agent_task.total)),
         wall_median:       ($ok|med(.wall_clock_secs)),
         agent_task:        ($ok|map(.tokens.agent_task.total)),
         wall:              ($ok|map(.wall_clock_secs))}' $files
}

base_sum="$(summarize baseline)"
warm_sum="$(summarize warm)"
jq -n --argjson base "$base_sum" --argjson warm "$warm_sum" --argjson n "$N" \
   '{n:$n, baseline:$base, warm:$warm}' > "$OUT/summary.json"

echo "================ A/B MEDIANS (N=$N) ================"
jq . "$OUT/summary.json"
echo "summary: $OUT/summary.json"
echo "AB_BATCH_DONE"
