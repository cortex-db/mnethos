---
title: Installation & Setup
slug: /
description: Install the Mnethos CLI, configure the Zsh plugin, and connect an AI provider to send your first prompt.
---

# Installation & Setup

Mnethos is a CLI-based coding harness — think Claude Code, but with first-class support for many AI providers. It works equally well with cloud models, open-weight models, and models running locally.

## Prerequisites

- A [Nerd Font](https://www.nerdfonts.com/) installed and enabled in your terminal (for example, [FiraCode Nerd Font](https://www.nerdfonts.com/font-downloads))
- [Zsh](https://github.com/ohmyzsh/ohmyzsh/wiki/Installing-ZSH) installed and configured

## Installation

### Step 1. Install the Mnethos binary

```bash
curl -fsSL https://mnethos.com/cli | sh
```

This works on macOS, Linux, Android, and Windows via WSL or Git Bash.

Verify the installation:

```bash
mnethos --help
```

### Step 2. Configure the Zsh plugin

Mnethos integrates with Zsh to let you send prompts directly from your shell prompt. Run the setup wizard:

```bash
mnethos zsh setup
```

Follow the interactive prompts. Once complete, **you must restart your terminal** for the plugin to take effect. Open a new terminal window, or reload the current session:

```bash
exec zsh
```

> **Important**
> The Zsh plugin will not be active until you restart your terminal. If the `:` prompt trigger isn't working, this is the most common cause.

If you're still having trouble, run the diagnostics command:

```bash
mnethos zsh doctor
```

This checks your environment and reports any configuration issues with the Zsh plugin.

### Step 3. Log in to an AI provider

Mnethos needs access to at least one AI model. Run:

```
:login
```

This walks you through selecting a provider and entering your API key.

**If you already have a [ChatGPT Plus](https://chatgpt.com/pricing) or [Claude](https://claude.com/pricing) subscription**, select the corresponding provider (OpenAI or Anthropic) and use that subscription's API access instead of buying a separate key.

> **Recommended providers**
>
> - [OpenRouter](https://openrouter.ai/) — one key, 300+ models from every major vendor
> - [OpenAI](https://platform.openai.com/) — GPT Codex series
> - [Anthropic](https://console.anthropic.com/) — Claude Sonnet and Opus series

> **Recommended models**
>
> - **Proprietary:** Claude Sonnet & Opus series, GPT Codex series
> - **Open-source:** GLM, Kimi, Minimax

After logging in, pick a model:

```
:model
```

Browse the list, type to filter, and press Enter. Mnethos remembers your choice across sessions. You can change it anytime.

### Step 4. Send your first prompt

With the Zsh plugin active and the LLM provider set up, type `:` followed by a **space** and your prompt:

```
: Hi! What is the time?
```

Mnethos takes it from there.

### Step 5. Explore available commands

To see all available Mnethos commands, type `:` and press `Tab` (without space):

```
: # then press Tab WITHOUT space
```

This lists every command you can run directly from your shell.

## Next Steps

Once you're set up, enable [Mnethos Services](/docs/mnethos-services/) for enhanced codebase understanding, tool-call guardrails, and a semantic search engine — no API key required.
