---
title: Custom Config Directory
slug: /mnethos-config
description: Relocate the Mnethos config directory with the MNETHOS_CONFIG environment variable.
---

# Custom Config Directory

By default, Mnethos keeps its configuration at `~/.mnethos/` on macOS/Linux and `%USERPROFILE%\.mnethos` on Windows. That stops working when you want the config in a dotfiles repo, on a different volume, or switched per environment.

`MNETHOS_CONFIG` points Mnethos at a different directory.

## What It Controls

When `MNETHOS_CONFIG` is unset, Mnethos reads from `~/.mnethos/.mnethos.toml`.

Set it to a directory path and Mnethos uses that path instead:

```bash
export MNETHOS_CONFIG=~/.config/mnethos
```

Mnethos will look for `~/.config/mnethos/.mnethos.toml`. If the directory or `.mnethos.toml` doesn't exist yet, Mnethos starts with defaults — and `:config-edit` creates the directory and file for you on first use.

## When to Change It

**Dotfiles repo:**

```bash
export MNETHOS_CONFIG=~/.config/mnethos
```

**Multiple environments** — switch configs for work vs personal:

```bash
export MNETHOS_CONFIG=~/.config/mnethos-work
```

**Different volume** — home directory is full or slow:

```bash
export MNETHOS_CONFIG=/data/mnethos
```

## Setting It

**`~/.env` — Mnethos-only, persistent:**

```bash
MNETHOS_CONFIG=~/.config/mnethos
```

**`~/.zshrc` — system-wide, persistent:**

```bash
export MNETHOS_CONFIG=~/.config/mnethos
```

Reload after editing: `source ~/.zshrc`

**Current session — temporary:**

```bash
export MNETHOS_CONFIG=~/test-config
```

## Verifying the Change

Check the variable:

```bash
echo $MNETHOS_CONFIG
```

Then run `:config-edit` in a Mnethos session. Your editor should open `$MNETHOS_CONFIG/.mnethos.toml`.

## Reverting to the Default

```bash
unset MNETHOS_CONFIG
```

Or remove the line from `~/.zshrc` and reload.

## Migrating an Existing Config

Move the directory:

```bash
mv ~/.mnethos ~/.config/mnethos
export MNETHOS_CONFIG=~/.config/mnethos
```

For the full list of settings inside the config file, see the [.mnethos.toml reference](/docs/mnethos-toml/).
