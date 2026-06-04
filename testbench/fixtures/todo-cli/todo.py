"""A tiny todo CLI.

Adding a new command (PROJECT CONVENTION):
  1. Implement a ``cmd_<name>(args)`` function.
  2. Register it in the ``COMMANDS`` dict below.
  3. Add a test for it in ``test_todo.py``.
  4. Document it in the "Usage" section of ``README.md``.
A command is only considered complete when all four steps are done.
"""

import sys

import storage


def cmd_add(args: list[str]) -> int:
    if not args:
        print("usage: todo add <text>")
        return 2
    todos = storage.load()
    next_id = max((t["id"] for t in todos), default=0) + 1
    todos.append({"id": next_id, "text": " ".join(args), "done": False})
    storage.save(todos)
    print(f"added #{next_id}")
    return 0


def cmd_list(args: list[str]) -> int:
    todos = storage.load()
    if not todos:
        print("(no todos)")
        return 0
    for t in todos:
        mark = "x" if t["done"] else " "
        print(f"[{mark}] #{t['id']} {t['text']}")
    return 0


# Register every command here (see the module docstring, step 2).
COMMANDS = {
    "add": cmd_add,
    "list": cmd_list,
}


def main(argv: list[str]) -> int:
    if not argv or argv[0] not in COMMANDS:
        print("usage: todo {" + "|".join(COMMANDS) + "} ...")
        return 2
    return COMMANDS[argv[0]](argv[1:])


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
