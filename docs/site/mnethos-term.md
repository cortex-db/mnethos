---
title: Terminal Context with `$MNETHOS_TERM`
slug: /mnethos-term
description: Give Mnethos automatic terminal context — recent commands and their exit codes — through the MNETHOS_TERM environment variable.
---

# Terminal Context with `$MNETHOS_TERM`

When you run `: fix this` after a failed command, Mnethos has no idea what you just ran. It can't see your last `cargo build` output or know that `npm test` exited with code 1. You end up narrating your terminal back to an agent sitting right next to it. That's backwards.

`MNETHOS_TERM` is on by default. The zsh plugin tracks the commands you run — what they were, whether they succeeded, and when — and passes that history to Mnethos every time you invoke `:`. The agent knows what you ran and what failed without you explaining any of it.

This changes the interaction from:

```
# you have to reconstruct what already happened: the build failed with a type error in src/main.rs on line 42 because...
```

to:

```
cargo build  # fails
: fix it     # Mnethos already knows what failed
```

## Disabling It

Context capture is on by default. To turn it off for the current session:

```bash
export MNETHOS_TERM=false
```

To disable it permanently, add to `~/.env` (Mnethos-only) or `~/.zshrc` (shell-wide):

```bash
MNETHOS_TERM=false
```

Reload after editing: `source ~/.zshrc`

To re-enable:

```bash
unset MNETHOS_TERM
```

## Verifying It Works

Check the variable is set:

```bash
echo $MNETHOS_TERM
```

Then run a command and ask Mnethos about it:

```
cargo build
: what just failed?
```

If Mnethos references the command you just ran, context capture is working.

## Controlling Buffer Size

`MNETHOS_TERM_MAX_COMMANDS` sets how many commands the plugin keeps in the buffer. The default is `5`.

```bash
export MNETHOS_TERM_MAX_COMMANDS=20
```

A larger buffer gives Mnethos more history to work with, at the cost of a larger context window. If your prompts are hitting model context limits, lower it. If you work in long pipelines where Mnethos needs to see further back, raise it.

## What Comes Next

Once `MNETHOS_TERM` is set, `: fix it` means exactly what it says — Mnethos knows the last thing that broke without you narrating it. The rest of the zsh integration — agent switching, file tagging, conversation management — is covered in [ZSH Support](/docs/zsh-support/).
