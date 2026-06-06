# testbench — memory A/B harness

Runs the mnethos agent on a realistic task **inside an isolated copy** of a
fixture project, with long-term memory (the `remember` + `mem_search` tools)
either ON or OFF, so we can measure whether memory helps against real data.

```
testbench/
  fixtures/todo-cli/      pristine test project (NEVER modified by the agent)
  tasks/                  task prompts fed to the agent
  run-test.sh             harness: copy → isolate → run → collect
  ab-batch.sh             health-gated A/B batch (N samples, medians)
  runs/                   per-run metrics + diffs (gitignored)
```

## Why this fixture/task

`fixtures/todo-cli` is a small Python CLI with two deliberately memory-worthy
properties the agent must *discover*:

1. **A multi-step convention** — adding a command requires 4 coordinated edits
   (implement → register in `COMMANDS` → add a test → update README). Easy to
   half-do; ideal "how" to remember.
2. **A non-obvious rule with a reason** — storage must use an atomic write
   because of a past corruption bug. A classic "why" worth remembering.

Run 1 (`add-done-command`) makes the agent discover these. Run 2
(`add-delete-command`) is where memory of run 1 would pay off — that contrast is
the point of the experiment.

## Prerequisites

- A built dev binary: `cargo build --bin mnethos`
- Provider auth configured (see `mnethos provider`)
- `python3` + `pytest` on PATH (the agent runs the project's tests)

## Run

```bash
bash testbench/run-test.sh                                   # default: add-done-command
bash testbench/run-test.sh testbench/tasks/add-delete-command.md
```

The fixture stays pristine; each run gets a fresh isolated git repo under
`$TMPDIR/mnethos-testruns/`. Metrics + the model's diff are mirrored into
`testbench/runs/<task>-<timestamp>/` for inspection.

## Inspect a run

Each run mirrors its results into `testbench/runs/<task>-<timestamp>/`:

```bash
cat testbench/runs/*/metrics.json     # wall-clock + agent_task tokens + files-changed
less testbench/runs/*/model.diff       # the change the agent made
```
