use forge_domain::{ContextMessage, Conversation};

/// Separate, dedicated assembly point for the Mnethos memory-recall context.
///
/// This is intentionally INDEPENDENT of [`SystemPrompt`](crate::system_prompt),
/// which builds forgecode's initial context via `Context::set_system_messages`
/// (AGENTS.md rules, skills, extensions, environment). Memory must never modify
/// or overwrite that. Instead, recalled memories are appended here as their OWN
/// separate message block, so the two context sources stay fully decoupled —
/// cleaner for us and for upstream sync.
///
/// It runs as a builder step in [`ForgeApp::chat`](crate::ForgeApp) AFTER the
/// system/user prompt are assembled but BEFORE `Orchestrator::run()`. Placement
/// matters: `run()` clones the working context up-front, so a context mutation
/// made from a `Start` hook would be discarded — but a mutation made here, before
/// `run()`, is preserved.
///
/// Retrieval/recall is NOT implemented yet, so [`Self::build_block`] returns
/// `None` and this step is a structural no-op. When recall lands, fill
/// `build_block` and the separate block flows in automatically, still without
/// touching `system_prompt.rs`.
#[derive(Default)]
pub struct MemoryContext;

impl MemoryContext {
    /// Creates a new memory-context builder.
    pub fn new() -> Self {
        Self
    }

    /// Renders the memory-recall block to inject, or `None` when there is
    /// nothing to inject. Empty until real recall is implemented.
    fn build_block(&self, _conversation: &Conversation) -> Option<String> {
        // TODO(memory): when recall exists, render recalled memories here, e.g.
        //   Some(format!("<memory_recall>\n{rendered}\n</memory_recall>"))
        None
    }

    /// Appends the memory block as its OWN separate message, leaving the existing
    /// system prompt (`system_messages`) untouched. No-op when there is nothing
    /// to inject.
    pub fn apply(self, mut conversation: Conversation) -> Conversation {
        if let Some(block) = self.build_block(&conversation) {
            if let Some(context) = conversation.context.as_mut() {
                context.messages.push(ContextMessage::system(block).into());
            }
        }
        conversation
    }
}
