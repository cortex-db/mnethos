---
title: Shortcuts
slug: /shortcuts
description: Reference for the built-in Zsh line-editor keyboard shortcuts available in your Mnethos session.
---

# Shortcuts

These shortcuts are built into ZSH — Mnethos doesn't add or modify them. They work in any ZSH session, not just when using Mnethos.

ZSH uses **Emacs keybindings by default**. If you prefer Vi mode, add `bindkey -v` to your `~/.zshrc`.

Run `mnethos zsh keyboard` at any time to print this reference in your terminal. For the full reference, see the [official ZSH Line Editor documentation](https://linux.die.net/man/1/zshzle).

## Navigation

| Shortcut   | Action                     |
|------------|----------------------------|
| `Ctrl+A`   | Move to beginning of line  |
| `Ctrl+E`   | Move to end of line        |
| `Option+F` | Move forward one word      |
| `Option+B` | Move backward one word     |

## Editing

| Shortcut   | Action                    |
|------------|---------------------------|
| `Ctrl+U`   | Kill line before cursor   |
| `Ctrl+K`   | Kill line after cursor    |
| `Ctrl+W`   | Kill word before cursor   |
| `Option+D` | Kill word after cursor    |
| `Ctrl+Y`   | Yank (paste) killed text  |
| `Ctrl+_`   | Undo last edit            |

## History

| Shortcut         | Action                          |
|------------------|---------------------------------|
| `Ctrl+R`         | Search command history backward |
| `Ctrl+S`         | Search command history forward  |
| `Ctrl+P` / `↑`   | Previous command                |
| `Ctrl+N` / `↓`   | Next command                    |
| `Option+<`       | Move to first history entry     |
| `Option+>`       | Move to last history entry      |

## Other

| Shortcut   | Action                  |
|------------|-------------------------|
| `Ctrl+L`   | Clear screen            |
| `Ctrl+C`   | Cancel current command  |
| `Ctrl+Z`   | Suspend current command |
| `Tab`      | Complete command/path   |

If `Option` key shortcuts aren't working, run `mnethos zsh doctor` — the most common cause is a terminal that isn't passing the Option key through correctly.

## Reference

ZSH exposes the full set of bindings and editor actions directly from the shell.

List all current key bindings:

```bash
bindkey
```

List all available editor actions:

```bash
zle -al
```

List bindings for a specific keymap (e.g. Emacs):

```bash
bindkey -M emacs
```
