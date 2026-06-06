---
title: "Skip the Copy-Paste: Reference Any Code Instantly"
slug: /vscode-extension
description: Reference code and launch Mnethos sessions directly from VS Code with the Mnethos extension.
---

# Skip the Copy-Paste: Reference Any Code Instantly

Tired of manually typing file paths and copying code snippets when asking Mnethos for help? This VS Code extension lets you reference any code with a single keystroke and start Mnethos sessions directly from your editor.

> **Note**
> The Mnethos VS Code extension (`Mnethos.mnethos-vscode`) is published under the `Mnethos` publisher. If you cannot find it in the Marketplace yet, install the [Mnethos CLI](/docs/) and use the in-terminal workflow described below.

## What This Extension Does

**The problem:** Describing code problems is slow and unclear

- "That function around line 50 something..."
- Copy-pasting code snippets manually
- Typing out long file paths

**The solution:** Show Mnethos exactly what you mean

- Select any code → Press `Ctrl+U` → Get a perfect reference
- Works with single lines, code blocks, or entire files

> **Prerequisite: Mnethos CLI**
> This extension works with the [Mnethos CLI](/docs/). Install that first if you haven't already.

## See It in Action

Select code → Press `Ctrl+U` → Reference copied and ready to use with Mnethos.

## Installation

### What You Need

- **VS Code**: Version 1.102.0 or higher
- **Mnethos CLI**: [Install](/docs/) it first if you haven't already

### Install the Extension

**Option 1: VS Code Marketplace (Recommended)**

1. Open VS Code
2. Press `Ctrl+Shift+X` to open Extensions panel
3. Search for **"Mnethos"**
4. Click **Install** on the official Mnethos extension

**Option 2: Command Line**

```bash
code --install-extension Mnethos.mnethos-vscode
```

**Option 3: Mnethos CLI**

```bash
mnethos vscode install-extension
```

**Test that it works:** Open any code file, select some text, press `Ctrl+U`. If a file reference gets copied to your clipboard, you're ready to go!

**Official extension:** [VS Code Marketplace](https://marketplace.visualstudio.com/items?itemName=Mnethos.mnethos-vscode)

## Basic Usage

### The Core Workflow

1. **Select code** (or don't select anything for whole file)
2. **Press `Ctrl+U`**
3. **Paste into Mnethos** conversation

That's it. No typing, no manual copying.

**What gets copied:**

Format: `@[<filepath>:<line start>:<line end>]`

**How selection works:**

- **No selection**: `@[path/to/file.js]` → References entire file
- **Single line**: `@[path/to/file.js:42:42]` → References line 42 only
- **Multiple lines**: `@[path/to/file.js:15:28]` → References lines 15-28

### Power Move: Multi-File References

Here's the real power move. You can reference multiple files in a single Mnethos prompt by copying and pasting references one at a time:

**Example:**

```
: Compare these approaches @[src/utils/oldMethod.js:15:45] @[src/utils/newMethod.js:20:50]
: Review this component and its styles @[components/Button.tsx] @[styles/button.css:12:34]
```

Now Mnethos can see exactly what code you're talking about, with full context and precise line numbers. No more "that function around line 50 something" conversations.

**Alternative ways to copy:**

- **Command Palette**: `Ctrl+Shift+P` → type "Copy File Reference"
- **Right-click Menu**: Select code → right-click → "Copy File Reference"

### Start New Mnethos Session

Start a Mnethos terminal directly in VS Code without switching windows.

**How to use:**

- **Command Palette**: `Ctrl+Shift+P` → type "Start New Mnethos Session"
- **Right-click Menu**: Right-click in any file → "Start New Mnethos Session"
- **Editor Toolbar**: Click the Mnethos icon in the top-right of the editor

The extension will:

1. Create a new integrated terminal in VS Code
2. Navigate to your workspace directory
3. Start Mnethos automatically
4. Auto-paste any file reference from the current editor (if open)

## Real-World Examples

### Debugging Issues

**Scenario:** Authentication fails in production but works locally.

```
: Help me debug this auth function @[src/auth/AuthService.ts:45:67] - works locally but fails in production
```

Mnethos sees the exact code and can suggest environment-specific issues to check.

### Code Reviews

**Scenario:** You spot a component that could be improved.

```
: Can you refactor this component to use hooks? @[components/UserProfile.tsx:12:89] Also suggest performance optimizations
```

Instead of generic advice, Mnethos sees your specific component and suggests targeted improvements.

### Type Mismatches

**Scenario:** API data doesn't match your TypeScript types.

```
: @[src/api/users.js:156:203] @[src/types/User.ts] have a type mismatch - API returns 'user_id' but type expects 'userId'
```

Mnethos compares both files simultaneously and suggests proper fixes.

### Feature Implementation

**Scenario:** Adding dark mode across multiple components.

```
: Add dark mode support to @[src/components/ThemeProvider.tsx:15:45] and update @[src/styles/theme.css:23:67]
```

Mnethos understands your theme system and suggests consistent changes across files.

## Configuration

The extension provides several settings to customize its behavior. Access settings via **File → Preferences → Settings → Extensions → Mnethos**.

### Available Settings

#### `mnethos.terminalMode`

Controls terminal interaction when copying file references with `Ctrl+U`.

- **Type**: String (dropdown)
- **Options**:
  - `once` (default): Open terminal once and reuse it for subsequent operations
  - `never`: Never open terminal, only copy file reference to clipboard
- **Default**: `once`

**When to use `once`:** Most users should use this. The extension intelligently manages Mnethos terminals, creating one when needed and reusing it for subsequent file references.

**When to use `never`:** Use this if you always work with Mnethos in an external terminal and only want the extension to copy file references to clipboard without any terminal interaction.

#### `mnethos.pasteDelay`

Delay in milliseconds before auto-pasting file reference into a newly created Mnethos terminal.

- **Type**: Number
- **Default**: `5000` (5 seconds)
- **Range**: 0-10000 milliseconds

**Why this exists:** When the extension creates a new Mnethos terminal, it needs time to start up before accepting input. This delay ensures the file reference is pasted after Mnethos is ready.

**When to adjust:**

- If file references aren't being pasted: Increase the value (try 7000-10000ms)
- If you have a fast machine and want quicker pasting: Decrease the value (try 3000-4000ms)
- Note: This only affects newly created terminals, not existing ones

#### `mnethos.showInstallationPrompt`

Show a prompt to install Mnethos CLI if it's not detected in your PATH.

- **Type**: Boolean
- **Default**: `true`

**When to disable:** If you use Mnethos via `npx` or have it installed in a non-standard location, you might want to disable this prompt.

## Troubleshooting

### Extension Not Working

**Quick fixes to try:**

1. Verify Mnethos CLI is installed - run `mnethos --version` in terminal
2. Check your VS Code version (Help → About) - need 1.102.0+
3. Verify the extension is enabled in Extensions view (`Ctrl+Shift+X`)
4. Restart VS Code completely

### Keyboard Shortcut Not Working

**`Ctrl+U` does nothing:**

This usually means another extension grabbed that shortcut. Here's how to fix it:

1. **File → Preferences → Keyboard Shortcuts**
2. Search for **"Mnethos"** or **"Copy File Reference"**
3. Click the pencil icon next to "Copy File Reference"
4. Pick a new combo like `Ctrl+Shift+U` or `Alt+U`

**Common culprits:** Vim extensions, browser preview extensions, other developer tools.

### Nothing Gets Copied to Clipboard

Try these workarounds:

- Use Command Palette: `Ctrl+Shift+P` → "Copy File Reference"
- Use right-click menu → "Copy File Reference"
- Check if you're in a text file (extension won't work on images/binaries)
- Some systems have clipboard permission issues - restart VS Code usually fixes this

### Terminal Not Opening or Mnethos Not Starting

**If "Start New Mnethos Session" doesn't work:**

1. Check that Mnethos is in your PATH - run `which mnethos` (macOS/Linux) or `where mnethos` (Windows) in terminal
2. Verify Mnethos is installed by running `mnethos --version`
3. Restart VS Code after installing Mnethos

**If file references aren't being pasted automatically:**

- Increase the `mnethos.pasteDelay` setting (try 7000-10000ms for slower machines)
- The default is 5000ms (5 seconds) - if that's not enough, Mnethos may need more time to start
- Check that your terminal shell is compatible (bash, zsh, PowerShell work well)
- Try using `Ctrl+U` to copy the reference, then manually paste it into the Mnethos terminal

### Weird File Paths

Sometimes you'll see paths like `/workspaces/project/src/file.js` instead of normal ones.

**This is actually fine.** VS Code uses different path formats for remote development, containers, and workspace setups. Mnethos handles these correctly, so don't worry about how they look.

## Pro Tips

**Start small:** Reference one function, ask Mnethos about it, see how it responds. Then level up to multi-file references.

**Be selective:** Don't dump entire files unless you actually need context from the whole thing. Mnethos works better with focused references.

**Combine with descriptions:** `@[component.tsx:45:67] this validation logic isn't working with empty strings` gives Mnethos both code and context.

## Next Steps

You're ready to start using the extension! The goal isn't to reference every line of code - it's to give Mnethos just enough context to actually help with your specific situation.

## Related Guides

- [File Tagging](/docs/file-tagging/)
- [Quickstart: Get started with Mnethos in minutes](/docs/)

## Still Need Help?

**If you're still stuck:**

- **Extension logs:** View → Output → Select "Mnethos" from dropdown
- **Report bugs:** [GitHub Issues](https://github.com/cortex-db/mnethos/issues)
- **Community help:** Join our [Discord](https://discord.gg/kRZBPpkgwq) for quick answers
