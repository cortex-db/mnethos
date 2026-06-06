---
title: Editor Configuration
slug: /editor-configuration
description: Compose prompts in your preferred editor with the `:edit` command, configured through MNETHOS_EDITOR or EDITOR.
---

# Editor Configuration

Typing a five-line prompt in a single-line input is miserable. The `:edit` command lets you compose prompts in a real editor — VS Code, Vim, Neovim, nano, whatever you prefer — then sends the result to Mnethos when you save and close.

```
:edit
```

Your editor opens a temporary file. Write your prompt, save, close. Mnethos reads the file and sends it as if you had typed it inline.

## Setting Your Editor

Mnethos checks two environment variables, in order:

| Variable         | Scope        | Example Value |
|------------------|--------------|---------------|
| `MNETHOS_EDITOR` | Mnethos only | `code --wait` |
| `EDITOR`         | System-wide  | `vim`         |

**`MNETHOS_EDITOR` takes priority.** If it's set, `EDITOR` is ignored. If neither is set, Mnethos falls back to `nano`.

`MNETHOS_EDITOR` exists for one reason: your preferred system editor and your preferred prompt editor might not be the same. Maybe you use `vim` for quick system edits but want VS Code for longer prompts. Set `MNETHOS_EDITOR` to decouple the two.

### VS Code

VS Code needs the `--wait` flag so Mnethos knows when you're done editing. Without it, the `code` command returns immediately and Mnethos sends an empty prompt.

```bash
export MNETHOS_EDITOR="code --wait"
```

### Vim / Neovim

Vim and Neovim run inside your terminal and block until you quit, so no extra flags are needed:

```bash
export EDITOR="vim"
# or
export EDITOR="nvim"
```

### nano

```bash
export EDITOR="nano"
```

### Other Editors

Any editor that blocks the calling process until the file is closed will work. The pattern is the same — if your editor returns immediately, look for a "wait" or "block" flag in its docs.

| Editor           | Command          |
|------------------|------------------|
| Sublime Text     | `subl --wait`    |
| IntelliJ IDEA    | `idea --wait`    |
| Zed              | `zed --wait`     |
| Emacs (GUI)      | `emacsclient -c` |
| Emacs (terminal) | `emacs -nw`      |

## Where to Set It

Three options, same trade-offs as any environment variable.

**`~/.env` — persistent, Mnethos-only**

Mnethos loads `~/.env` on every run. The variable is invisible to other tools:

```bash
# ~/.env
MNETHOS_EDITOR=code --wait
```

**`~/.zshrc` (or `~/.bashrc`) — persistent, system-wide**

Makes the variable available to everything in your shell:

```bash
# ~/.zshrc
export EDITOR="vim"
```

Reload your shell after editing (`source ~/.zshrc`) or open a new terminal.

**Current session — temporary**

```bash
export MNETHOS_EDITOR="code --wait"
```

Gone when the session ends.

## When to Use `:edit`

Inline prompts work fine for short requests. `:edit` earns its keep when:

- **The prompt has structure** — steps, lists, code snippets, or multi-paragraph context that's awkward to compose in a single line.
- **You want to iterate** — write a draft, re-read it, tighten it up before sending. A real editor with cursor movement, undo, and search makes this natural.
- **You're pasting content** — logs, stack traces, or code blocks are easier to arrange in an editor than at a prompt.

Pair it with [multiline input](/docs/zsh-support/#multiline-text) for shorter structured prompts, and reach for `:edit` when the prompt outgrows inline composition.
