---
title: permissions.yaml
slug: /permissions
description: Define allow, deny, and confirm policies for Mnethos's built-in tools, applied only when restricted mode is enabled.
---

# permissions.yaml

`permissions.yaml` is Mnethos's policy file for built-in tools. It only matters when [restricted mode](/docs/mnethos-toml/) is enabled in `.mnethos.toml`.

## Start with the simplest possible example

This policy does three things:

- allows reads anywhere
- asks before writes
- blocks `rm`

```yaml
policies:
  - permission: allow
    rule:
      read: "**/*"
  - permission: confirm
    rule:
      write: "**/*"
  - permission: deny
    rule:
      command: "rm*"
```

That is the whole mental model.

## Turn it on

`permissions.yaml` does nothing until restricted mode is enabled:

```toml
restricted = true
```

Add that to `~/.mnethos/.mnethos.toml`, or to the `.mnethos.toml` inside your custom config directory if you use `MNETHOS_CONFIG`.

When `restricted = false`, Mnethos behaves normally and does not gate tool execution through this policy file.

## Where the file lives

Mnethos reads the file from its config directory:

- macOS/Linux: `~/.mnethos/permissions.yaml`
- Windows: `%USERPROFILE%\.mnethos\permissions.yaml`
- Custom config directory: `$MNETHOS_CONFIG/permissions.yaml`

If restricted mode is enabled and the file does not exist yet, Mnethos creates it with a default allow-all policy.

That default looks like this:

```yaml
policies:
  - permission: allow
    rule:
      read: "**/*"
  - permission: allow
    rule:
      write: "**/*"
  - permission: allow
    rule:
      command: "*"
  - permission: allow
    rule:
      url: "*"
```

So turning on restricted mode alone does not make Mnethos stricter. **The restriction comes from the rules you write.**

## The shape of the file

Every file starts with one top-level key:

```yaml
policies:
  - permission: allow
    rule:
      read: "**/*.rs"
```

A simple policy has two parts:

- `permission`: what should happen
- `rule`: what should match

### Permission values

| Value | What it means |
|---|---|
| `allow` | Run the operation immediately |
| `deny` | Reject the operation immediately |
| `confirm` | Pause and ask you first |

### Rule types

A rule matches exactly one kind of operation:

| Key | Matches | Example |
|---|---|---|
| `read` | File reads and file searches | `"docs/**/*"` |
| `write` | Writes, patches, and deletes | `"src/**/*"` |
| `command` | Shell command strings | `"git *"` |
| `url` | Network fetches | `"https://api.github.com/*"` |

You can also scope any rule to a working directory with `dir`:

```yaml
- permission: allow
  rule:
    write: "**/*.rs"
    dir: "/home/user/project/*"
```

That rule only applies when the current working directory matches the `dir` glob.

## How Mnethos evaluates policies

A matching `allow` is not always final. Mnethos can keep scanning because a later `deny` or `confirm` may still need to stop the operation.

Suppose Mnethos wants to run `git status`.

```
run `git status`
  -> check rules from top to bottom
  -> matching deny?    stop and reject
  -> matching confirm? stop and ask
  -> matching allow?   remember it and keep going
  -> nothing decisive matched? ask by default
```

That last line matters: **no matching policy means `confirm`, not `allow`.**

## What Mnethos actually checks

Built-in tools are mapped into four operation types:

| Tool family | Checked as |
|---|---|
| `Read`, `FsSearch` | `read` |
| `Write`, `Patch`, `MultiPatch`, `Remove` | `write` |
| `Shell` | `command` |
| `Fetch` | `url` |

Some tools are exempt from this policy system, including `SemSearch`, `Undo`, `Plan`, and `Task`.

MCP tools also bypass this file entirely. `permissions.yaml` governs Mnethos's built-in tools, not external MCP integrations.

## Confirmation mode

When a matching rule returns `confirm` — or when nothing matches and Mnethos falls back to `confirm` — you get a prompt with three choices:

| Choice | Result |
|---|---|
| Accept | Allow this one operation |
| Reject | Deny this one operation |
| Accept and Remember | Allow it now and append a matching rule to `permissions.yaml` |

The remembered rule depends on what you approved:

| Operation | Generated pattern |
|---|---|
| Read or write `file.rs` | `*.rs` |
| Fetch `https://example.com/api` | `example.com*` |
| Run `git push origin main` | `git push*` |
| Run `ls` | `ls*` |
| Read or write a file with no extension | No rule is added |

This makes confirmation useful for tightening policies gradually instead of designing the whole file up front.

## Logical policies

Simple rules cover most setups. When they do not, `permissions.yaml` also supports `all`, `any`, and `not`.

```yaml
policies:
  - all:
      - permission: allow
        rule:
          read: "src/**/*"
      - permission: allow
        rule:
          dir: "/home/user/project/*"
          read: "**/*"
  - any:
      - permission: allow
        rule:
          read: "**/*.rs"
      - permission: allow
        rule:
          read: "**/*.toml"
  - not:
      permission: deny
      rule:
        command: "rm -rf/*"
```

Use these sparingly. Most policy files are easier to reason about when each rule does one obvious thing.

## Good patterns

Here are a few narrower patterns.

### Allow writes only for one kind of file

```yaml
policies:
  - permission: allow
    rule:
      read: "**/*"
  - permission: allow
    rule:
      write: "**/*.rs"
  - permission: deny
    rule:
      write: "**/*"
```

### Allow one API, deny the rest

```yaml
policies:
  - permission: allow
    rule:
      url: "https://api.github.com/*"
  - permission: deny
    rule:
      url: "*"
```

### Allow writes only inside one project directory

```yaml
policies:
  - permission: allow
    rule:
      write: "**/*"
      dir: "/home/user/myproject/*"
  - permission: deny
    rule:
      write: "**/*"
```

## Common mistakes

### Turning on restricted mode and expecting instant safety

Restricted mode only enables policy evaluation. If the generated `permissions.yaml` still allows everything, Mnethos still allows everything.

### Forgetting the fallback behavior

If no rule matches, Mnethos asks. That is usually what you want, but it can feel surprising if you expected silent denial.

### Trying to control MCP tools here

You cannot. This file covers built-in tools only.

## Where to go next

If you have not enabled restricted mode yet, start with the `.mnethos.toml` setting in the [.mnethos.toml reference](/docs/mnethos-toml/).

Then come back and write the smallest policy file that matches how you actually work.
