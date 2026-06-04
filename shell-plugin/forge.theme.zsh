#!/usr/bin/env zsh

# Enable prompt substitution for RPROMPT
setopt PROMPT_SUBST

# Model and agent info with token count
# Fully formatted output directly from Rust
# Returns ZSH-formatted string ready for use in RPROMPT
function _forge_prompt_info() {
    local forge_bin="${_MNETHOS_BIN:-${MNETHOS_BIN:-mnethos}}"
    
    # Get fully formatted prompt from forge (single command).
    # Pass session model/provider as CLI flags when set so the rprompt
    # reflects the active session override rather than global config.
    local -a forge_cmd
    forge_cmd=("$forge_bin")
    forge_cmd+=(zsh rprompt)
    [[ -n "$_MNETHOS_SESSION_MODEL" ]] && local -x MNETHOS_SESSION__MODEL_ID="$_MNETHOS_SESSION_MODEL"
    [[ -n "$_MNETHOS_SESSION_PROVIDER" ]] && local -x MNETHOS_SESSION__PROVIDER_ID="$_MNETHOS_SESSION_PROVIDER"
    [[ -n "$_MNETHOS_SESSION_REASONING_EFFORT" ]] && local -x MNETHOS_REASONING__EFFORT="$_MNETHOS_SESSION_REASONING_EFFORT"
    _MNETHOS_CONVERSATION_ID=$_MNETHOS_CONVERSATION_ID _MNETHOS_ACTIVE_AGENT=$_MNETHOS_ACTIVE_AGENT COLUMNS=$COLUMNS "${forge_cmd[@]}" 2>/dev/null
}

# Right prompt: agent and model with token count (uses single forge prompt command)
# Set RPROMPT if empty, otherwise append to existing value
if [[ -z "$_MNETHOS_THEME_LOADED" ]]; then
    RPROMPT='$(_forge_prompt_info)'"${RPROMPT:+ ${RPROMPT}}"
fi
