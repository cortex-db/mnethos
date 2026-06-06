---
title: Operating Agents
slug: /operating-agents
description: Choose between Mnethos's architect, smith, and sage agents for planning, implementation, and research.
---

# Operating Agents

Mnethos provides three specialized agents, each designed for different stages of development work. They differ in capabilities and access levels, allowing you to choose the right approach for your task.

## Agent Comparison

| Agent       | Access       | Purpose                  | Best For                                                     |
|-------------|--------------|--------------------------|-------------------------------------------------------------|
| `architect` | read         | Planning & analysis      | Reviewing impact, planning changes, critical systems        |
| `smith`     | read + write | Implementation           | Making changes, fixing bugs, creating features              |
| `sage`      | read         | Research & investigation | Understanding codebases, tracing bugs, analyzing architecture |

**Typical workflow**: Use **`architect`** to plan → Switch to **`smith`** to implement

Any agent can lean on **`sage`** to research and understand your codebase when needed.

## Agent Selection Summary

Here are the key points to remember when selecting an agent:

### How to switch quickly

1. Type `:agent` in your Mnethos session
2. Browse the available agents list
3. Use ↑/↓ to choose an agent
4. Press Enter to confirm

### Why selection matters

Models control raw intelligence, while agents control behavior and execution style. Picking the right agent gives you help that matches your current stage of work.

### When to switch

- Use **`sage`** for deep research and system understanding
- Use **`architect`** for planning and change analysis
- Use **`smith`** for direct implementation and code changes
- Use **custom agents** for team- or domain-specific workflows

### Pro tips

- Your conversation and project context are preserved when switching agents
- Combine `:agent` with `:model` to tune both behavior and intelligence

---

## `architect` Agent

`architect` analyzes your codebase and creates detailed implementation plans. It proposes solutions and explains the impact of changes without modifying your code, writing its plans to the `plans/` directory.

**Switch to `architect`**: `:architect` (alias `:plan`)

**Ideal for:**

- Planning complex refactoring
- Understanding scope before implementation
- Working with critical or production code
- Learning how to implement specific features
- Changes requiring team review

**Example prompts:**

- "How would you redesign this API for better scalability?"
- "Create a plan to add user authentication"
- "What's needed to implement pagination?"

---

## `smith` Agent

`smith` implements solutions directly. It modifies files, creates code, and executes commands to complete tasks immediately. It is the default agent.

**Switch to `smith`**: `:smith` (active by default)

**Ideal for:**

- Quick fixes and routine tasks
- Refactoring with immediate results
- Implementing approved plans
- Tasks where you want hands-off execution
- Creating new features

**Example prompts:**

- "Fix the null pointer exception in UserService"
- "Create a React component for the user profile"
- "Add unit tests for the payment processor"

---

## `sage` Agent

`sage` is a read-only research agent. It investigates code, traces functionality, and analyzes architecture without making any changes. The other agents can also draw on `sage`-style research when they need deeper codebase insights.

**Switch to `sage`**: `:sage` (alias `:ask`)

**Ideal for:**

- Understanding unfamiliar codebases
- Tracing bugs and following data flow
- Analyzing architecture
- Answering questions about how the code works

---

## Switching Between Agents

You can switch between agents at any time during your session:

- Use `:architect` (or `:plan`) to switch to the `architect` agent
- Use `:smith` to switch to the `smith` agent
- Use `:sage` (or `:ask`) to switch to the `sage` agent
- Use `:agent` to see all available agents and choose from a dropdown

**Common patterns:**

- Use **`architect`** before making significant changes to critical systems
- Switch to **`smith`** when you're ready to implement
- Use **`sage`** for research whenever you need to understand the code first

**Best practice**: Use version control and commit your work before using `smith` for significant changes.

---

## Related Guides

- [Plan and Act Guide: Strategic AI Development Workflow with Mnethos](/docs/plan-and-act-guide/)
- [AI Model Selection Guide: Optimize Mnethos for Your Workflow](/docs/model-selection-guide/)
