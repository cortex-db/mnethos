---
title: File Tagging
slug: /file-tagging
description: Attach files, directories, and images as prompt context using `@` tags with TAB completion.
---

# File Tagging

File tagging lets you attach project context directly in your prompt with `@` references.

> **Press `TAB` to complete file tags**
>
> Type `@` followed by a partial file or directory name, then press **`TAB`**. A fuzzy picker opens — type to filter, arrow keys to navigate, `Enter` to select. The full path is inserted automatically.

```
: explain the logic in @src/utils/helper<TAB>
: review @package<TAB>
```

`.gitignore` is respected; ignored paths won't appear in the list.

## What you can tag

Files can be ignored using [Ignoring Files](/docs/ignoring-files/). Ignored files and directories are not listed in tagging suggestions.

### Files

Tag a file to give Mnethos direct code context:

```
@[src/auth/AuthService.ts]
```

### Directories

Tag a directory when you want to work across a folder:

```
@[src/components]
```

This is useful when your task spans multiple related files.

### Images

Tag images for visual context (UI states, mockups, diagrams):

```
@[assets/button-states.png]
@[docs/wireframes/user-journey.jpg]
```

Supported formats include PNG, JPG, JPEG, SVG, and WebP.

## Why use tagging

Tagged files are auto-attached to the prompt, so the agent gets context immediately. This saves a round trip where you would otherwise need to re-send or paste content manually.

## Important limitation

Be careful when tagging very large files. Extremely large files can fail to attach due to size limits.

When that happens, tag smaller scopes instead:

- Use a more focused file
- Use line ranges like `@[src/auth/AuthService.ts:120:180]`
- Split the task across multiple smaller tags
