---
title: Mnethos Services
slug: /mnethos-services
description: Enable Mnethos Services for the context engine, tool-call guardrails, and semantic search across your codebase.
---

# Mnethos Services

Mnethos Services is the runtime layer that helps the model stay on trajectory while it explores, edits, and executes tools.

## What it does

These are the most visible capabilities, not the full feature set.

- **Context engine**: Beats SOTA across retrieval benchmarks, uses up to 93% fewer tokens, and stays fast while starting the agent in the most relevant files and functions.
- **Tool-call guardrails**: Catches invalid arguments, common tool-call mistakes, then auto-corrects them before they fail.
- **Skill engine**: Assists the model in choosing the right skill for the job, so task-specific guidance is applied at the right time.

**There is nothing to configure here.** After you enable it, it keeps running in the background.

## Enable Mnethos Services

Run:

```
:login
```

Then select **Mnethos Services** in the provider list and complete browser authentication.

No API key required — sign in with Google or GitHub.

## Enable semantic sync for your project

Run:

```
:sync
```

This indexes your project and enables `sem_search`.

To monitor indexing progress and see which files are being synced, run:

```
:sync-status
```

## Ignoring files

Files can be ignored using [Ignoring Files](/docs/ignoring-files/).

If a file is ignored, Mnethos Services excludes it from sync, and the context engine cannot use that file for retrieval.

## Verify services are active

Run:

```
:tools
```

Look for `sem_search` under `SYSTEM`.

## Disable Mnethos Services

Run:

```
:logout
```

This signs you out and disables Mnethos Services.

To enable again later, run `:login`, select **Mnethos Services**, then run `:sync` for the project you want indexed.

## Data & Privacy

### What gets sent to Mnethos servers

When you run `:sync`, Mnethos Services indexes your project on Mnethos's servers. The indexing pipeline is:

1. **Chunking** — your source files are split into segments
2. **Embedding** — each chunk is converted into a vector embedding
3. **Storage** — the original file chunk and its embedding are stored

Mnethos stores the file content and its corresponding embedding. Nothing else.

### What does not reach Mnethos servers

Your LLM provider is configured by you and called directly from your device. Prompts, completions, and any conversation context go from your machine to your chosen LLM — they do not pass through Mnethos's infrastructure.

### How the data is used

Stored files and embeddings are used exclusively to power `sem_search` — retrieving the most relevant context for your queries. Mnethos does not use your code to train models, does not sell it, and does not share it with any third party.

### Deleting your synced data

Logging out (`:logout`) signs you out but does not delete your synced data. To remove indexed data, use the workspace commands:

```
:workspace
```

This gives you full control to inspect and delete synced workspaces. You can remove all indexed data for a project at any time.

### Files you want to exclude

Use [Ignoring Files](/docs/ignoring-files/) to prevent specific files or directories from ever being sent to Mnethos's servers. Ignored files are excluded from sync entirely.
