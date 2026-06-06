---
title: SKILL.md
slug: /skills
description: Teach Mnethos reusable workflows by writing them in SKILL.md files that load automatically when a task calls for them.
---

# SKILL.md

Skills are reusable workflows you teach Mnethos once. Write the process down in a `SKILL.md` file and place any supporting scripts, examples, or other resources alongside it — Mnethos will automatically load the right skill whenever the task calls for it.

## Getting Started

Skills can live in three places:

- **Project skills** — `.mnethos/skills/<skill-name>/SKILL.md` inside your project, checked into version control and shared with your team.
- **Agents skills** — `~/.agents/skills/<skill-name>/SKILL.md` on your machine, shared with any agent tool that follows the common agents convention.
- **Global skills** — `~/.mnethos/skills/<skill-name>/SKILL.md` on your machine, available across every project you work on.

Each skill is a plain markdown file — write it the same way you'd explain the process to a new teammate.

```
.mnethos/                        # project skills (highest precedence)
└── skills/
    └── release-notes/
        └── SKILL.md
~/.agents/                       # agents skills (shared across agent tools)
└── skills/
    └── release-notes/
        └── SKILL.md
~/.mnethos/                      # global skills (all projects)
└── skills/
    └── release-notes/
        └── SKILL.md
```

When multiple sources define a skill with the same name, the one with higher precedence wins: **project > agents > global > built-in**.

Here's what a release notes skill looks like:

````markdown
# Generate Release Notes

1. Run `./scripts/get-commits.sh` to collect commits since the last tag
2. Run `./scripts/categorize.sh` to group them into Features, Bug Fixes, and Breaking Changes
3. Write the release notes in `CHANGELOG.md` using the output from the scripts
4. Run `./scripts/validate-changelog.sh` to confirm the format is correct
````

Mnethos reads all skills at the start of a session and automatically applies the relevant one based on what you're asking it to do — no need to invoke them by name.

The easiest way to create a skill is to ask Mnethos directly. Describe the workflow — the steps, scripts, and edge cases — and it will generate the `SKILL.md` in the right place:

```
Create a release-notes skill. It should collect all commits since the last tag,
group them by type — Features, Bug Fixes, Breaking Changes — write the notes to
the changelog, and run a validation check at the end.
```

Review the generated file, adjust anything that doesn't match your setup, and it's ready to use. The more detail you give, the better the skill.

## Importing Claude Code Skills

**Skills are fully compatible with [Claude Code](https://code.claude.com/docs/en/skills).** The `SKILL.md` format is identical — no conversion needed.

If you already have skills in a Claude Code project, copy them straight into Mnethos:

```bash
cp -r .claude/skills .mnethos/skills
```

They work without any changes.

## Verifying Your Skills

To confirm Mnethos has picked up your skills, run `:skill` in the chat. You'll see a list of all available skills along with their descriptions.

```
:skill
```
