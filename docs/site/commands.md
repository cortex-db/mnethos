---
title: Custom Commands
slug: /commands
description: Turn repeatable workflows into named Mnethos commands stored as Markdown files.
---

# Custom Commands

You repeat the same workflow constantly — lint, test, fix, commit. Every time, you type it out or paste it from a notes file. Custom commands let you turn any repeatable workflow into a named command that Mnethos executes on demand.

## How It Works

A custom command is a Markdown file in the `.mnethos/commands/` directory. The filename becomes the command name. The file body is the instruction Mnethos follows when you invoke it.

```
.mnethos/
└── commands/
    ├── check.md     →  :check
    └── fixme.md     →  :fixme
```

Type `:check` in the chat, and Mnethos runs whatever `check.md` describes. That's it.

## File Format

Every command file has two parts: a frontmatter block and an instruction body.

```markdown
---
name: check
description: Checks if the code is ready to be committed
---

- Run the `lint` and `test` commands and verify if everything is fine.
  <lint>cargo +nightly fmt --all; cargo +nightly clippy --fix --allow-staged --allow-dirty --workspace</lint>
  <test>cargo insta test --accept --unreferenced=delete</test>
- Fix every issue found in the process
```

### Frontmatter

| Field         | Required | Description                                       |
| ------------- | -------- | ------------------------------------------------- |
| `name`        | Yes      | The command name (e.g. `check` → `:check`)        |
| `description` | Yes      | One-line summary shown in the command picker      |

### Body

The body is plain Markdown. Write it the same way you'd explain the workflow to a teammate. You can use:

- **Prose** for context or decision logic
- **Bullet lists** for sequential steps
- **XML-style tags** to attach literal shell commands to a step (e.g. `<lint>...</lint>`, `<test>...</test>`)
- **Code blocks** for multi-line scripts

Mnethos reads the body as instructions and executes them. If a step fails, it attempts to fix the problem before continuing — just like it would for any other task.

## A Minimal Example

The simplest possible command:

```markdown
---
name: fixme
description: Looks for all the fixme comments in the code and attempts to fix them
---

Find all the FIXME comments in source-code files and attempt to fix them.
```

Invoke it with `:fixme` and Mnethos searches every source file for `FIXME` comments and tries to resolve each one.

## A More Complex Example

Commands can embed exact shell commands so Mnethos runs the right tools every time:

```markdown
---
name: check
description: Checks if the code is ready to be committed
---

- Run the `lint` and `test` commands and verify if everything is fine.
  <lint>cargo +nightly fmt --all; cargo +nightly clippy --fix --allow-staged --allow-dirty --workspace</lint>
  <test>cargo insta test --accept --unreferenced=delete</test>
- Fix every issue found in the process
```

The `<lint>` and `<test>` tags tell Mnethos the exact commands to run for those steps. If clippy reports an error, Mnethos fixes it. If a test fails, Mnethos investigates. You don't have to tell it how — the command already knows.

## Where to Put Commands

Commands can live in three places, loaded in precedence order:

```
.mnethos/commands/        ← project commands (highest precedence)
~/.agents/commands/       ← shared across agent tools
~/.mnethos/commands/      ← global, across all projects
```

Project commands are the most common. Check them into version control and your team shares the same `:check`, `:fixme`, and any other workflows you define.

## Invoking Commands

Type the command name with a leading colon in the Mnethos chat:

```
:check
:fixme
```

Mnethos picks it up immediately. No restart needed — new command files are available as soon as they exist on disk.

## Verifying Your Commands

To see all available commands, run `:help` in the chat. You'll get a list with names and descriptions.

```
:help
```

The hardest part of getting value from custom commands is identifying which workflows deserve one. A good rule: if you've typed the same instruction three times, write a command for it.
