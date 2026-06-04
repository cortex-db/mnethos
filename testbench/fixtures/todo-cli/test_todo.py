"""Tests for the todo CLI.

Each command must have a test here (see the convention in ``todo.py``).
Tests isolate the database via the ``TODO_DB`` env var so they never touch a
real ``todos.json``.
"""

import todo


def test_add_then_list(tmp_path, monkeypatch, capsys):
    monkeypatch.setenv("TODO_DB", str(tmp_path / "todos.json"))

    assert todo.main(["add", "buy", "milk"]) == 0
    assert "added #1" in capsys.readouterr().out

    assert todo.main(["list"]) == 0
    out = capsys.readouterr().out
    assert "#1 buy milk" in out
    assert "[ ] #1" in out


def test_list_empty(tmp_path, monkeypatch, capsys):
    monkeypatch.setenv("TODO_DB", str(tmp_path / "todos.json"))

    assert todo.main(["list"]) == 0
    assert "(no todos)" in capsys.readouterr().out


def test_unknown_command(tmp_path, monkeypatch):
    monkeypatch.setenv("TODO_DB", str(tmp_path / "todos.json"))
    assert todo.main(["frobnicate"]) == 2
