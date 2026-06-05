#!/usr/bin/env zsh

# Configuration variables for mnethos plugin
# Using typeset to keep variables local to plugin scope and prevent public exposure

typeset -h _MNETHOS_BIN="${MNETHOS_BIN:-mnethos}"
typeset -h _MNETHOS_CONVERSATION_PATTERN=":"
typeset -h _MNETHOS_MAX_COMMIT_DIFF="${MNETHOS_MAX_COMMIT_DIFF:-100000}"

typeset -h _MNETHOS_COMMANDS=""

# Hidden variables to be used only via the Mnethos CLI
typeset -h _MNETHOS_CONVERSATION_ID
typeset -h _MNETHOS_ACTIVE_AGENT

# Previous conversation ID for :conversation - (like cd -)
typeset -h _MNETHOS_PREVIOUS_CONVERSATION_ID

# Session-scoped model and provider overrides (set via :model / :m).
# When non-empty, these are passed as --model / --provider to every mnethos
# invocation for the lifetime of the current shell session.
typeset -h _MNETHOS_SESSION_MODEL
typeset -h _MNETHOS_SESSION_PROVIDER

# Session-scoped reasoning effort override (set via :reasoning-effort / :re).
# When non-empty, exported as MNETHOS_REASONING__EFFORT for every mnethos invocation.
typeset -h _MNETHOS_SESSION_REASONING_EFFORT

# Terminal context capture settings
# Master switch for terminal context capture (preexec/precmd hooks)
typeset -h _MNETHOS_TERM="${MNETHOS_TERM:-true}"
# Maximum number of commands to keep in the ring buffer (metadata: cmd + exit code)
typeset -h _MNETHOS_TERM_MAX_COMMANDS="${MNETHOS_TERM_MAX_COMMANDS:-5}"
# OSC 133 semantic prompt marker emission: "auto", "on", or "off"
typeset -h _MNETHOS_TERM_OSC133="${MNETHOS_TERM_OSC133:-auto}"
# Ring buffer arrays for context capture
typeset -ha _MNETHOS_TERM_COMMANDS=()
typeset -ha _MNETHOS_TERM_EXIT_CODES=()
typeset -ha _MNETHOS_TERM_TIMESTAMPS=()
