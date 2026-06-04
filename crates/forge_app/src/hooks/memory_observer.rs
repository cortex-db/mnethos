use std::path::PathBuf;

use async_trait::async_trait;
use forge_domain::{AgentId, Conversation, EndPayload, EventData, EventHandle, ModelId};
use serde::Serialize;
use tracing::{info, warn};

/// PoC observability hook for the future Mnethos memory layer.
///
/// On conversation end — the point where a real "consolidation" step will
/// eventually run — this serializes the *entire* conversation (messages, tool
/// calls, reasoning, token usage, and metrics such as files touched and todos)
/// to a JSON snapshot under `.mnethos/consolidation/`. The goal is to inspect
/// exactly what raw material a consolidation step would receive, *before*
/// building any retrieval, summarization, or embedding.
///
/// Observation only: it never mutates the conversation and never fails the
/// turn. It is opt-in via the `MNETHOS_MEMORY_OBSERVE` environment variable, so
/// normal runs are completely unaffected.
#[derive(Debug, Clone, Default)]
pub struct MemoryObserver;

impl MemoryObserver {
    /// Creates a new memory observer.
    pub fn new() -> Self {
        Self
    }
}

/// Snapshot envelope written to disk for inspection.
#[derive(Serialize)]
struct ConsolidationSnapshot<'a> {
    /// Wall-clock time of observation (milliseconds since the UNIX epoch).
    observed_at_ms: u128,
    /// The agent that performed the work.
    agent_id: &'a AgentId,
    /// The model that performed the work.
    model_id: &'a ModelId,
    /// The complete conversation available at consolidation time.
    conversation: &'a Conversation,
}

#[async_trait]
impl EventHandle<EventData<EndPayload>> for MemoryObserver {
    async fn handle(
        &self,
        event: &EventData<EndPayload>,
        conversation: &mut Conversation,
    ) -> anyhow::Result<()> {
        // Opt-in: stay completely inert unless explicitly enabled.
        if std::env::var_os("MNETHOS_MEMORY_OBSERVE").is_none() {
            return Ok(());
        }

        // Observation must never break a turn — log and bail on any error.
        if let Err(err) = write_snapshot(event, conversation) {
            warn!(error = %err, "memory-observer: failed to write consolidation snapshot");
        }

        Ok(())
    }
}

/// Serializes the conversation snapshot to `.mnethos/consolidation/<id>-<ts>.json`.
fn write_snapshot(event: &EventData<EndPayload>, conversation: &Conversation) -> anyhow::Result<()> {
    let observed_at_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);

    let snapshot = ConsolidationSnapshot {
        observed_at_ms,
        agent_id: &event.agent.id,
        model_id: &event.model_id,
        conversation,
    };

    let dir = PathBuf::from(".mnethos").join("consolidation");
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{}-{}.json", conversation.id, observed_at_ms));
    std::fs::write(&path, serde_json::to_string_pretty(&snapshot)?)?;

    let message_count = conversation
        .context
        .as_ref()
        .map(|context| context.messages.len())
        .unwrap_or(0);

    info!(
        path = %path.display(),
        messages = message_count,
        files_touched = conversation.metrics.files_accessed.len(),
        "memory-observer: consolidation snapshot written"
    );

    Ok(())
}
