---
title: Create an Agent
slug: /creating-agents
description: Define custom Mnethos agents as markdown files with YAML frontmatter to control tools, models, and behavior.
---

# Create an Agent

A custom agent is a markdown file with a YAML header. That's it. Write the system prompt the way you'd brief a skilled contractor on their role, and Mnethos will use it every time you invoke that agent.

## Where agents live

Mnethos looks in two places:

| Location            | Scope                       | Use when                                       |
| ------------------- | --------------------------- | ---------------------------------------------- |
| `~/.mnethos/agents/`| Global — all projects       | General-purpose agents you'll reuse everywhere |
| `.mnethos/agents/`  | Project — current repo only | Agents specific to one codebase or team        |

Project agents take priority. If both locations have an agent with the same `id`, the project one wins.

Create the directory before adding your first agent:

```bash
# Global
mkdir -p ~/.mnethos/agents

# Project-specific
mkdir -p .mnethos/agents
```

## Your first agent

The minimum viable agent needs exactly one thing: a unique `id`.

Create `~/.mnethos/agents/security-auditor.md`:

```markdown
---
id: security-auditor
title: Security Auditor
description: Reviews code for vulnerabilities and recommends fixes
tools:
  - read
  - search
---

You are a security specialist focused on finding and fixing vulnerabilities.

Review code for injection flaws, authentication gaps, insecure data handling, and dependency risks. For every issue found, explain the risk and provide a specific fix with a code example.
```

Restart Mnethos, then run `:agent` to see your new agent in the list. It is also automatically added as a `:` command you can invoke directly.

## The file anatomy

Every agent definition is a markdown file split into two parts:

```markdown
---
# YAML frontmatter — capabilities and metadata
id: my-agent
title: My Agent
---

System prompt — the agent's instructions, written in plain markdown.
```

**The frontmatter** controls what the agent can do: which tools it has, which model it uses, how it samples. **The system prompt** controls how it thinks and responds.

The `id` must be unique across all your agents. The filename doesn't matter — only the `id` field is used for identification.

## Configuring tools

By default, agents have no tools unless you specify them. Restrict access to exactly what the agent needs. Run `:tools` in a Mnethos session to see every tool available in your environment:

```yaml
tools:
  - read # Read files and directories
  - write # Create and modify files
  - patch # Apply targeted changes
  - shell # Execute shell commands
  - search # Search within files
  - fetch # Retrieve external resources
  - remove # Delete files
  - undo # Reverse previous changes
```

Use `*` to grant access to every available tool:

```yaml
tools:
  - "*" # All tools
```

> **Tip**
> In practice, every tool definition is injected into the model's context. Granting all tools — especially when many MCP servers are configured — consumes significant context space and leaves less room for your actual work. Prefer explicit tool lists or narrow globs.

For MCP integrations, use a prefix glob so new MCP servers are automatically included:

```yaml
tools:
  - read
  - search
  - "mcp_*" # All MCP tools — database, browser, APIs, etc.
```

A security auditor only needs `read` and `search`. A deployment agent needs `shell`. Match the tool list to the role — agents with fewer tools make fewer unintended changes.

## Model and behavior settings

```yaml
---
id: my-agent
title: My Agent
description: Brief description of what this agent does

# Model selection (optional — defaults to your configured model)
model: claude-sonnet-4
provider: anthropic # Must be snake_case: open_router, openai, requesty, etc.

# Sampling (optional)
temperature: 0.1 # 0.0–2.0 — lower = more precise, higher = more creative
top_p: 0.9 # 0.0–1.0 nucleus sampling threshold
top_k: 40 # 1–1000
max_tokens: 8192 # 1–100,000

# Limits (optional)
max_turns: 50 # Max conversation turns before the agent stops
max_requests_per_turn: 10
max_tool_failure_per_turn: 3 # Max tool failures per turn before forcing completion

# Visibility (optional)
tool_supported: true # Whether this agent can be called as a tool by other agents

# Reasoning (optional — for models that support it)
reasoning:
  enabled: true
  effort: medium # low | medium | high
  max_tokens: 2048 # Must be > 1024 and < max_tokens
  exclude: false # Hide reasoning output from the response
---
```

Keep `temperature` low (0.05–0.2) for agents that write code or follow strict rules. Use higher values only for agents doing creative or exploratory work.

## Shaping user messages

`user_prompt` lets you wrap or augment every incoming user message before it reaches the model. It runs as a Handlebars template with these variables:

| Variable           | Value                                                        |
| ------------------ | ----------------------------------------------------------- |
| `{{event.name}}`   | `task` for the first message, `feedback` for subsequent ones |
| `{{event.value}}`  | The raw user input                                          |
| `{{current_date}}` | Today's date                                                |

Use it to inject structured context the model should always see — like a timestamp or a consistent envelope format:

```yaml
user_prompt: |-
  <{{event.name}}>{{event.value}}</{{event.name}}>
  <system_date>{{current_date}}</system_date>
```

With this template, the first user message `fix the bug` becomes:

```
<task>fix the bug</task>
<system_date>2026-04-01</system_date>
```

And a follow-up becomes:

```
<feedback>looks good, but also handle the edge case</feedback>
<system_date>2026-04-01</system_date>
```

Use `|-` (block scalar, strip trailing newline) rather than `|` to avoid sending a spurious blank line at the end of every message.

## Customizing built-in agents

You can override Mnethos's built-in agents (`smith`, `architect`, `sage`) by creating an agent file with a matching `id`. The built-in definition is replaced entirely.

Create `.mnethos/agents/smith-frontend.md`:

```markdown
---
id: "smith"
title: "Frontend Smith"
description: "Smith agent tuned for React and TypeScript"
tools:
  - read
  - write
  - patch
  - shell
temperature: 0.1
---

You are a frontend development expert for this React TypeScript project.

Build modern, accessible components. Explain architectural decisions. Include TypeScript types in every example you write.
```

This override applies only to the project — global Mnethos sessions are unaffected.

## Troubleshooting

**Agent doesn't appear in `:agent` list**

- Check the file has a `.md` extension
- Verify the frontmatter is valid YAML (spaces, not tabs for indentation)
- Confirm the `id` is unique — duplicate IDs cause the second agent to be silently skipped
- Restart Mnethos after adding new agent files

**YAML parse errors**

Quote strings that contain colons, brackets, or other special characters:

```yaml
title: "Backend: API Expert" # colon in value requires quotes
description: "Expert [Node.js]" # brackets require quotes
```

Use `|` for multiline strings:

```yaml
description: |
  Analyzes code for security vulnerabilities,
  explains each risk, and provides concrete fixes.
```

**Agent used as a tool by other agents isn't recognized**

Agents can only be invoked as tools by other agents if they have a `description` field. An agent without `description` is available in the `:agent` picker but not as a callable tool.

## Related

- [Agents](/docs/operating-agents/) — built-in agents and when to use each
- [SKILL.md](/docs/skills/) — teach Mnethos reusable workflows
- [MCP Integration](/docs/mcp-integration/) — connect agents to external services
- [AGENTS.md Guide](/docs/custom-rules-guide/) — project-wide rules for all agents
