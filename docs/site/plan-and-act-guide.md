---
title: Plan First, Then Act
slug: /plan-and-act-guide
description: Use the Mnethos architect (planning) and smith (implementation) agents together to plan first, then act on complex work.
---

# Plan First, Then Act: Strategic AI Development Workflow with Mnethos

One of the biggest mistakes developers make with AI coding tools is jumping straight into implementation. After analyzing thousands of successful AI-assisted development sessions, we've learned that the most productive workflow follows a simple pattern: **Plan first, then act**.

Mnethos makes this workflow smooth with two specialized agents designed to work together.

## Meet Your AI Development Team

### Architect Agent: Your Strategic Planner

Architect operates in **read-only mode**, making it perfect for analysis and planning without touching your code:

- Analyzes your codebase and identifies potential issues
- Creates detailed implementation plans
- Explores different solution approaches
- Reviews code for security, performance, and architecture concerns

**When to use Architect:**

- Before making significant changes to critical systems
- When you need to understand the scope and impact of a task
- For architecture planning
- When working in unfamiliar codebases

### Smith Agent: Your Implementation Partner

Smith has **full read-write access** and handles the actual implementation:

- Modifies files and creates new code
- Executes commands and runs tests
- Implements the solutions from your plan
- Provides real-time feedback as changes are made

**When to use Smith:**

- After reviewing and approving a plan from Architect
- For routine tasks you're confident about
- When you want hands-off implementation
- For quick fixes with proper version control

## The Plan-and-Act Workflow

Here's how successful developers use both agents together:

### 1. Start with Architect for Planning

Switch to Architect from your ZSH shell:

```
:architect
```

Ask Architect to create a detailed plan:

```
: Write a plan for adding rate limiting to our API. Include:
- Which endpoints need protection
- Storage mechanism for rate data
- Error responses and status codes
- Integration points with existing middleware
Now critique this plan. What did you miss?
```

### 2. Review and Refine the Plan

Architect will provide a structured plan and then critique it for gaps. Review this carefully - a good plan eliminates most of implementation confusion later.

### 3. Switch to Smith for Implementation

Switch back to Smith:

```
:smith
```

Reference the plan and start implementation:

```
: Following the $(@rate-limiting-plan.md) we discussed, implement the Redis-based rate limiter for the /api/auth endpoints first in $(@src/auth).
```

### 4. Iterate as Needed

Switch back to Architect if you encounter complex decisions, then return to Smith for continued implementation.

## Why This Works

**Planning prevents confusion:** When AI understands the full scope upfront, it makes better implementation decisions and avoids getting lost halfway through.

**Separation of concerns:** Architect focuses purely on analysis without the pressure to implement, leading to better strategic thinking.

**Safety first:** Critical systems get proper review before any changes are made.

**Faster iteration:** Once you have a solid plan, Smith can implement quickly without constant back-and-forth.

## Quick Tips for Success

- **Be specific in your planning requests** - include edge cases, error handling, and integration points
- **Commit frequently** - clean git state makes it easier to track AI changes
- **Review everything** - treat AI output like a junior developer's code
- **Avoid frequent agent switching** - it causes context thrashing, hurts cache performance, and creates confusing context handoffs

Remember: You're the lead, `architect` is your strategic advisor, and `smith` is your implementation partner. Use each for what they do best.
