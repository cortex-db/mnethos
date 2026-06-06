---
title: Custom Binary Path
slug: /mnethos-bin
description: Point the Mnethos ZSH plugin at a specific binary with the MNETHOS_BIN environment variable.
---

# Custom Binary Path

When you install Mnethos normally, the binary lands in your `$PATH` as `mnethos`. The ZSH plugin assumes that name and calls it directly. That works until it doesn't — when you're testing a local build, running a binary at an absolute path, or keeping multiple versions side by side.

`MNETHOS_BIN` lets you tell the ZSH plugin exactly which binary to use instead.

## What It Controls

The ZSH plugin captures `MNETHOS_BIN` when it loads and uses it for every Mnethos command it runs on your behalf — commits, conversation management, command completion, path formatting, and more. Internally the plugin resolves the binary once:

```bash
_MNETHOS_BIN="${MNETHOS_BIN:-mnethos}"
```

Change it and you change which binary answers. The default is `mnethos` — whatever `which mnethos` resolves to on your system.

## When to Change It

**Local build from source.** You've compiled Mnethos locally and want to test your changes without installing the binary system-wide:

```bash
export MNETHOS_BIN=/path/to/mnethos/target/debug/mnethos
```

**Non-standard install path.** The binary is on disk but not in a directory on your `$PATH`:

```bash
export MNETHOS_BIN=/opt/mnethos/bin/mnethos
```

**Multiple versions.** You have a stable release as `mnethos` and want to test a nightly build without replacing it:

```bash
export MNETHOS_BIN=~/bin/mnethos-nightly
```

In each case, the ZSH plugin picks up the change and routes every invocation through the specified binary.

## Setting It

**`~/.zshrc` — persistent**

This is the right place for `MNETHOS_BIN`. It must be set before the ZSH plugin is loaded, so it belongs in your shell profile rather than `~/.env`:

```bash
# ~/.zshrc
export MNETHOS_BIN=/path/to/your/mnethos
# Load the ZSH integration. Use $MNETHOS_BIN so the same binary
# that runs your commands also generates the plugin.
eval "$($MNETHOS_BIN zsh plugin)"
```

Reload your shell after editing:

```bash
source ~/.zshrc
```

**Current session — temporary**

To switch binaries for just the current terminal session:

```bash
export MNETHOS_BIN=~/builds/mnethos-dev
eval "$($MNETHOS_BIN zsh plugin)"
```

The change disappears when the session ends. This is useful for one-off testing without touching your permanent configuration.

## Verifying the Change

After setting `MNETHOS_BIN`, confirm the right binary is being used:

```bash
echo $MNETHOS_BIN          # shows the path you set
$MNETHOS_BIN --version     # confirms the binary responds and shows its version
```

If `$MNETHOS_BIN --version` fails, the path is wrong or the binary isn't executable. Double-check the path and run `chmod +x $MNETHOS_BIN` if needed.

## Reverting to the Default

Unset the variable to go back to the system-installed `mnethos`:

```bash
unset MNETHOS_BIN
eval "$(mnethos zsh plugin)"
```

Or remove the `export MNETHOS_BIN=...` line from your `~/.zshrc` and reload.

For everything else the ZSH integration can do — agent selection, multiline input, file tagging — see the [ZSH Support](/docs/zsh-support/) reference.
