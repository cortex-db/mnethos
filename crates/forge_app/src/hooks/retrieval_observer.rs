use std::path::PathBuf;

use async_trait::async_trait;
use forge_domain::{AgentId, Conversation, EventData, EventHandle, ModelId, StartPayload};
use serde::Serialize;
use tracing::{info, warn};

/// PoC observability hook for the *retrieval* side of the future Mnethos memory
/// layer.
///
/// Fires once at conversation start — the point where a real retrieval step
/// would recall relevant memories and inject them into the prompt. Retrieval
/// itself is **not implemented yet**, so this always records an empty recall.
/// It exists so the experiment surfaces *both* observation points from day one:
/// retrieval-in (here) and consolidation-in (`MemoryObserver`). When real
/// recall lands, this same seam will report what was recalled and injected.
///
/// NOTE: injecting recalled memories into the live model context cannot happen
/// here — `orch.rs` clones the working context before the Start event and
/// overwrites `conversation.context` from that clone inside the loop, so a Start
/// mutation would be discarded. Real injection will need a different seam. This
/// observer only *reads*, so it is unaffected.
///
/// Observation only: never mutates the conversation, never fails the turn.
/// Opt-in via the `MNETHOS_MEMORY_OBSERVE` environment variable.
#[derive(Debug, Clone, Default)]
pub struct RetrievalObserver;

impl RetrievalObserver {
    /// Creates a new retrieval observer.
    pub fn new() -> Self {
        Self
    }
}

/// Snapshot of the retrieval step, written to disk for inspection.
#[derive(Serialize)]
struct RetrievalSnapshot<'a> {
    /// Wall-clock time of observation (milliseconds since the UNIX epoch).
    observed_at_ms: u128,
    /// The agent about to perform the work.
    agent_id: &'a AgentId,
    /// The model about to perform the work.
    model_id: &'a ModelId,
    /// Whether a real memory store/recall is wired up. Always `false` for now.
    retrieval_implemented: bool,
    /// The text a real retrieval step would search on (latest message), if any.
    query_preview: Option<String>,
    /// Memories recalled and injected. Always empty until recall is built.
    recalled: Vec<serde_json::Value>,
    /// Human note about the current (no-op) state.
    note: &'static str,
}

#[async_trait]
impl EventHandle<EventData<StartPayload>> for RetrievalObserver {
    async fn handle(
        &self,
        event: &EventData<StartPayload>,
        conversation: &mut Conversation,
    ) -> anyhow::Result<()> {
        if std::env::var_os("MNETHOS_MEMORY_OBSERVE").is_none() {
            return Ok(());
        }

        if let Err(err) = write_snapshot(event, conversation) {
            warn!(error = %err, "memory-observer: failed to write retrieval snapshot");
        }

        Ok(())
    }
}

/// Serializes the retrieval snapshot to `.mnethos/retrieval/<id>-<ts>.json`.
fn write_snapshot(event: &EventData<StartPayload>, conversation: &Conversation) -> anyhow::Result<()> {
    let observed_at_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);

    // What a real retrieval step would query on: the most recent message text.
    let query_preview = conversation.context.as_ref().and_then(|context| {
        context
            .messages
            .iter()
            .rev()
            .find_map(|entry| entry.message.content())
            .map(|content| content.chars().take(400).collect::<String>())
    });

    let snapshot = RetrievalSnapshot {
        observed_at_ms,
        agent_id: &event.agent.id,
        model_id: &event.model_id,
        retrieval_implemented: false,
        query_preview,
        recalled: Vec::new(),
        note: "retrieval not implemented yet: no memory store, nothing recalled or injected",
    };

    let dir = PathBuf::from(".mnethos").join("retrieval");
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{}-{}.json", conversation.id, observed_at_ms));
    std::fs::write(&path, serde_json::to_string_pretty(&snapshot)?)?;

    info!(
        path = %path.display(),
        recalled = 0,
        "memory-observer: retrieval snapshot written (recall not implemented; empty)"
    );

    Ok(())
}
