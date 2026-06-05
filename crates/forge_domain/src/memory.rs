//! Long-term memory domain: wire types for the ai-working-memory gRPC contract
//! and the [`MemoryRepository`] port. Mirrors the `WorkspaceIndexRepository`
//! (context-engine) shape: a thin async port implemented by the repo layer.

use std::path::Path;

use anyhow::Result;
use async_trait::async_trait;
// eserde::Deserialize (not serde's) so these types can nest inside the
// eserde-derived `Remember` tool-input struct in the ToolCatalog.
use eserde::Deserialize;
use schemars::JsonSchema;
use serde::Serialize;

/// Stable per-project memory session key: the working-directory basename. Shared
/// by the write path (remember tool) and the read path (retrieval hook) so both
/// address the same memory-service session. Derived from cwd (available at both
/// turn start and during tool execution).
pub fn memory_session_key(cwd: &Path) -> String {
    cwd.file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("default")
        .to_string()
}

/// One normalized English anchor of a concept (MemoryWrite.Anchor).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct MemoryAnchor {
    /// Single lowercase English lemma (one word), e.g. "schema".
    pub text: String,
    /// subject|predicate|object|qualifier|time
    pub role: String,
    /// 0..1
    pub strength: f32,
}

/// One atomic semantic unit of an episode (MemoryWrite.Concept).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct MemoryConcept {
    /// One atomic sub-claim (NOT the whole episode text).
    pub text: String,
    /// 0..1
    pub confidence: f32,
    /// 3-8 anchors; first should be the project name (role subject).
    pub anchors: Vec<MemoryAnchor>,
}

/// A caller-parsed episode written to long-term memory (MemoryWrite.Episode).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct MemoryEpisode {
    /// ONE self-contained English statement, opening with the project identity;
    /// stored AND embedded server-side.
    pub text: String,
    /// The atomic concepts inside this episode.
    pub concepts: Vec<MemoryConcept>,
}

/// Result of creating an episode (MemoryWrite.CreateEpisodeResponse).
#[derive(Debug, Clone, Default)]
pub struct MemoryWriteResult {
    pub episode_id: String,
    pub concept_count: i32,
    pub anchor_count: i32,
    pub unique_refs: i32,
    pub custom_l0_count: i32,
    pub reused_concepts: i32,
}

/// One recalled item (MemoryWrite.RetrievedItem) — mirrors retrieve.Result 1:1
/// (the same shape the MCP `retrieve_episodes` tool returns). May contain
/// contradictory facts by design; `created_at` lets the caller reason on recency.
#[derive(Debug, Clone)]
pub struct MemoryRecallItem {
    pub node_id: String,
    /// 0 = L0 backbone anchor, 1 = L1 concept, 2 = L2 episode.
    pub level: i32,
    pub activation: f32,
    /// RFC3339 timestamp (may be empty).
    pub created_at: String,
    /// The node's text/label(s): episode/concept statements live here.
    pub aliases: Vec<String>,
    /// L1 concept confidence; `None` for episodes / L0.
    pub confidence: Option<f32>,
}

/// Port to the external graph-memory service (ai-working-memory) over gRPC.
///
/// The implementor resolves its own connection config (server URL + token) — the
/// caller passes only domain data, exactly like [`crate::WorkspaceIndexRepository`].
/// A missing/!configured memory backend yields empty/zeroed results, never an
/// error, so memory stays a no-op when unconfigured.
#[async_trait]
pub trait MemoryRepository: Send + Sync {
    /// Persist ONE episode into the caller's session (server get-or-creates the
    /// session from `session_key`). `Ok(None)` when memory is not configured.
    async fn create_episode(
        &self,
        session_key: &str,
        episode: MemoryEpisode,
    ) -> Result<Option<MemoryWriteResult>>;

    /// Targeted spreading-activation retrieval (the fast route = MCP
    /// `retrieve_episodes`). `anchors` are statement-shape English phrases.
    /// Empty Vec when nothing matches or memory is not configured.
    async fn retrieve(
        &self,
        session_key: &str,
        anchors: Vec<String>,
    ) -> Result<Vec<MemoryRecallItem>>;
}
