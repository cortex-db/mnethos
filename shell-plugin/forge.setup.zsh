# !! Contents within this block are managed by 'mnethos zsh setup' !!
# !! Do not edit manually - changes will be overwritten !!

# Add required zsh plugins if not already present
if [[ ! " ${plugins[@]} " =~ " zsh-autosuggestions " ]]; then
    plugins+=(zsh-autosuggestions)
fi
if [[ ! " ${plugins[@]} " =~ " zsh-syntax-highlighting " ]]; then
    plugins+=(zsh-syntax-highlighting)
fi

# Load mnethos shell plugin (commands, completions, keybindings) if not already loaded
if [[ -z "$_MNETHOS_PLUGIN_LOADED" ]]; then
    eval "$(mnethos zsh plugin)"
fi

# Load mnethos shell theme (prompt with AI context) if not already loaded
if [[ -z "$_MNETHOS_THEME_LOADED" ]]; then
    eval "$(mnethos zsh theme)"
fi
