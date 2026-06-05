use std::sync::Arc;

use forge_app::MemoryService;
use forge_domain::{MemoryEpisode, MemoryRecallItem, MemoryRepository};

/// [`MemoryService`] impl bridging to the repo-layer [`MemoryRepository`] (the
/// gRPC client). Mirrors `ForgeWorkspaceService` for the context engine. No-op
/// (0 written / empty recall) when memory is unconfigured — the repo handles that.
pub struct ForgeMemoryService<F> {
    infra: Arc<F>,
}

impl<F> ForgeMemoryService<F> {
    pub fn new(infra: Arc<F>) -> Self {
        Self { infra }
    }
}

#[async_trait::async_trait]
impl<F: MemoryRepository> MemoryService for ForgeMemoryService<F> {
    async fn remember(
        &self,
        session_key: &str,
        episodes: Vec<MemoryEpisode>,
    ) -> anyhow::Result<usize> {
        let mut written = 0usize;
        for episode in episodes {
            // Skip degenerate episodes (no statement) so we never silently report
            // a write for empty content the graph would drop.
            if episode.text.trim().is_empty() {
                continue;
            }
            if self.infra.create_episode(session_key, episode).await?.is_some() {
                written += 1;
            }
        }
        Ok(written)
    }

    async fn retrieve(
        &self,
        session_key: &str,
        queries: Vec<String>,
    ) -> anyhow::Result<Vec<MemoryRecallItem>> {
        self.infra.retrieve(session_key, queries).await
    }
}
