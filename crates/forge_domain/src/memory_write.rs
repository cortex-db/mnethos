use anyhow::Result;
use async_trait::async_trait;

use crate::ApiKey;

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

/// Repository for writing episodes to the external graph-memory service
/// (ai-working-memory) over gRPC, via the "memory" provider.
///
/// `server_url` and `auth_token` are resolved from the "memory" provider
/// credential (url param + api key) by the caller, not stored here — the memory
/// server lives at a different URL than the context-engine/forgecode services.
#[async_trait]
pub trait MemoryWriteRepository: Send + Sync {
    /// Create one episode in the memory service, scoped by `session_key`
    /// (the server does get-or-create of a session per (owner, session_key)).
    async fn create_episode(
        &self,
        server_url: &str,
        auth_token: &ApiKey,
        session_key: &str,
        episode: MemoryEpisode,
    ) -> Result<MemoryWriteResult>;
}
