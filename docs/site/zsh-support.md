---
title: ZSH Support
slug: /zsh-support
description: Send prompts to Mnethos from your native Zsh session using the `:` sentinel — no environment switch, no lost context.
---

# ZSH Support

Mnethos's interactive mode runs in its own environment. That means your ZSH aliases, custom functions, and shell tooling don't work inside it. You're stuck choosing between AI help and your own productivity setup.

The `:` sentinel character solves this. It lets you send prompts to Mnethos from your native ZSH session — no environment switch, no lost context, no broken aliases.

```zsh
# Your aliases work as usual (gst = git status, gcam = git commit -am)
gst
gcam "fix: resolve memory leak"

# Ask Mnethos without leaving your shell
: analyze the memory usage patterns in src/server.rs

# Run your tools as part of the investigation
ps aux | grep server
htop -p $(pgrep server)

# Continue with full context
: now optimize the memory allocations you identified in the server struct
```

Shell commands and AI prompts live in the same workflow. Context carries across both.

> **Press `TAB` after `:`**
>
> Type `:` then immediately press **`TAB`** to open the command completion list — switch agents, start a new conversation, open the editor, and more.

```
:<TAB>   # opens the full command list
```

## Examples

### Basic Prompts

```
: explain this error message
: refactor this function to be more readable
: add error handling to the database connection
```

Prompts go to your last-used agent. On your first interaction, Mnethos uses its default agent, `smith`. The conversation continues across prompts until you run `:new`.

### Agent Selection

Switch agents by prefixing the agent name after `:`:

```
:sage
```

Mnethos prints a confirmation and updates your terminal's right-hand prompt (RPROMPT):

```
⏺ [16:14:54] SAGE is now the active agent
```

All subsequent bare `:` prompts now go to sage:

```
: explain the algorithm complexity and performance characteristics
: what are the potential edge cases?
```

You can also pass a prompt inline as a shortcut — this switches the agent and sends the prompt in one step:

```
:smith refactor this function to be more maintainable
```

> **Don't know the agent name?**
>
> Run `:agent` to pick from a list of all configured agents.

### Starting a New Conversation

Mnethos carries conversation context forward indefinitely within a session, until a terminal window is closed. When you move to a different task and don't want the previous context bleeding in, run `:new`:

```
:new
```

This clears the conversation history and starts fresh. The active agent stays the same.

You can also pass a prompt directly — `:new` starts the fresh conversation and sends it in one step:

```
:new hi what's the time
```

### Switching Conversations

To switch to a different existing conversation, run `:conversation`:

```
:conversation
```

This opens a list of your saved conversations. Select one to switch to it.

To jump back to the last conversation you were in, use the `-` shorthand:

```
:conversation -
```

### File Tagging

Tag files in your prompts with `@` followed by a partial name, then press **`TAB`**:

```
: review the changes in @package<TAB>
: explain the logic in @src/utils/helper<TAB>
: optimize the queries in @database/queries<TAB>
```

> **Press `TAB` after `@` to pick a file**
>
> After typing `@` and a few characters, press **`TAB`** to open a fuzzy file picker. Type to filter, arrow keys to navigate, `Enter` to select. The full file path is inserted into your prompt automatically. `.gitignore` is respected.

If `fd` and `fzf` aren't installed, use the full path directly:

```
: review the changes in @[src/components/Header.tsx]
```

### Multiline Text

When your prompt needs structure (lists, steps, logs), insert line breaks directly in the prompt composer:

- **Windows/Linux:** `Shift+Enter`
- **macOS:** `Option+Enter`

This lets you write multiline prompts without sending early.

> **Tip**
> See [Keyboard Shortcuts](/docs/shortcuts/) for all shortcuts available in your ZSH session.

### `:edit` as an Alternative

For longer prompts, use `:edit` instead of typing everything inline:

```
:edit
```

This opens your configured editor from `$MNETHOS_EDITOR` or `$EDITOR`. Write your prompt, save, and close the editor to send it.

Example (VS Code):

```bash
export MNETHOS_EDITOR="code --wait"
# or: export EDITOR="code --wait"
```

### Retrying a Request

If you cancel a prompt mid-flight with `Ctrl+C` and want to run it again, use `:retry`:

```
:retry
```

This resends the last request without you having to retype it. Most useful after an accidental interrupt or a timeout.

### Editing Configuration

To open the Mnethos configuration file (`~/.mnethos/.mnethos.toml`) in your default editor, run `:config-edit`:

```
:config-edit
```

See the [configuration reference](/docs/mnethos-toml/) for a full list of available settings.

## Troubleshooting

Start with these two commands — they cover most issues:

```bash
mnethos zsh doctor # checks your environment and reports problems
mnethos zsh setup  # re-runs the ZSH integration setup
```

If both run cleanly and things still aren't working, join us on [Discord](https://discord.gg/kRZBPpkgwq) and we'll help you sort it out.
