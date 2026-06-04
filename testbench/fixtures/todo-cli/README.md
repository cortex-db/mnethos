# todo

A tiny command-line todo list. Todos are stored as JSON.

## Usage

    python todo.py add <text>     # add a new todo
    python todo.py list           # list all todos

## Storage

Todos persist to `todos.json` (override the location with the `TODO_DB` env
var). **All writes go through an atomic temp-file + rename**
(`storage._atomic_write`) because a crash mid-write previously corrupted the
database. Never write the DB file directly — reuse `storage.save()`.

## Adding a command (convention)

A command is only "done" when **all four** steps are complete:

1. Implement `cmd_<name>(args)` in `todo.py`.
2. Register it in the `COMMANDS` dict.
3. Add a test in `test_todo.py`.
4. Document it here under **Usage**.

## Running tests

    pytest -q
