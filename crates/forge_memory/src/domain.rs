//! Wire types for the memory gRPC contract (ai-working-memory MemoryWrite).
//! Plain data only — the gRPC calls live in [`crate::client`].

/// One normalized English anchor of a concept. Mirrors the memory service
/// contract (ai-working-memory MemoryWrite.Anchor).
#[derive(Debug, Clone)]
pub struct MemoryAnchor {
    pub text: String,
    /// subject|predicate|object|qualifier|time
    pub role: String,
    /// 0..1
    pub strength: f32,
}

/// One atomic semantic unit of an episode (MemoryWrite.Concept).
#[derive(Debug, Clone)]
pub struct MemoryConcept {
    pub text: String,
    /// 0..1
    pub confidence: f32,
    pub anchors: Vec<MemoryAnchor>,
}

/// A caller-parsed episode to write into long-term memory (MemoryWrite.Episode).
#[derive(Debug, Clone)]
pub struct MemoryEpisode {
    /// ONE English declarative statement; stored AND embedded server-side.
    pub text: String,
    pub concepts: Vec<MemoryConcept>,
}

/// Result of creating an episode (MemoryWrite.CreateEpisodeResponse).
#[derive(Debug, Clone)]
pub struct MemoryWriteResult {
    pub episode_id: String,
    pub concept_count: i32,
    pub anchor_count: i32,
    pub unique_refs: i32,
    pub custom_l0_count: i32,
    pub reused_concepts: i32,
}

/// One recalled item from the memory service (MemoryWrite.RetrievedItem).
///
/// Mirrors the memory service's `retrieve.Result` 1:1 — the SAME shape the MCP
/// `retrieve_episodes` tool returns. Recall MAY contain contradictory facts (an
/// old and a new value of the same thing) by design; `created_at` lets the
/// caller reason about recency.
#[derive(Debug, Clone)]
pub struct MemoryRecallItem {
    pub node_id: String,
    /// 0 = L0 backbone anchor, 1 = L1 concept, 2 = L2 episode.
    pub level: i32,
    /// Spreading-activation score (ranking; higher = more relevant).
    pub activation: f32,
    /// RFC3339 timestamp of when the fact was recorded (may be empty).
    pub created_at: String,
    /// The node's text/label(s): episode/concept statements live here.
    pub aliases: Vec<String>,
    /// L1 concept confidence (0..1); `None` for episodes / L0 anchors.
    pub confidence: Option<f32>,
}
