"""Persistent storage for todos (a JSON file)."""

import json
import os
import tempfile
from pathlib import Path


def _db_path() -> Path:
    """Location of the todo database (override with the TODO_DB env var)."""
    return Path(os.environ.get("TODO_DB", "todos.json"))


def load() -> list[dict]:
    path = _db_path()
    if not path.exists():
        return []
    with path.open("r", encoding="utf-8") as f:
        return json.load(f)


def save(todos: list[dict]) -> None:
    # IMPORTANT: never write the DB file directly. A crash mid-write once left a
    # truncated JSON file and corrupted the database, so every write goes through
    # an atomic temp-file + rename. Preserve this invariant in any new storage
    # code.
    _atomic_write(_db_path(), json.dumps(todos, indent=2))


def _atomic_write(path: Path, data: str) -> None:
    parent = str(path.parent) if str(path.parent) not in ("", ".") else "."
    fd, tmp = tempfile.mkstemp(dir=parent, suffix=".tmp")
    try:
        with os.fdopen(fd, "w", encoding="utf-8") as f:
            f.write(data)
        os.replace(tmp, path)  # atomic on POSIX
    except BaseException:
        if os.path.exists(tmp):
            os.unlink(tmp)
        raise
