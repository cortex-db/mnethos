//! Workspace persistence and vector search for the context engine.
//!
//! A workspace is a single indexed codebase. It owns a set of files (tracked by
//! content hash so the client can sync incrementally) and the embedded chunks
//! derived from those files. Search is a brute-force cosine scan over a
//! workspace's chunks — adequate for single-tenant, per-codebase indexes where
//! a workspace holds at most tens of thousands of chunks.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::error::ServerError;

/// A file tracked in a workspace, identified by content hash.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileHash {
    /// Path relative to the workspace root.
    pub path: String,
    /// Content hash the client uses to detect changes.
    pub hash: String,
}

/// An embedded chunk stored in a workspace.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StoredChunk {
    /// Stable identifier for the chunk node.
    pub node_id: String,
    /// Path of the source file.
    pub path: String,
    /// Verbatim chunk text.
    pub content: String,
    /// 1-based first line.
    pub start_line: u32,
    /// 1-based last line.
    pub end_line: u32,
    /// Embedding vector for the chunk.
    pub embedding: Vec<f32>,
}

/// A chunk returned from a search, scored against the query.
#[derive(Debug, Clone, PartialEq)]
pub struct ScoredChunk {
    /// The matched chunk.
    pub chunk: StoredChunk,
    /// Cosine similarity in `[-1, 1]` (higher is more relevant).
    pub relevance: f32,
    /// Cosine distance `1 - relevance` (lower is more relevant).
    pub distance: f32,
}

/// Metadata describing a workspace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceInfo {
    /// Unique workspace identifier.
    pub id: String,
    /// Absolute working directory the workspace indexes.
    pub working_dir: String,
    /// When the workspace was created.
    pub created_at: DateTime<Utc>,
    /// When the workspace was last modified, if ever.
    pub last_updated: Option<DateTime<Utc>>,
}

/// Search parameters for [`WorkspaceStore::search`].
#[derive(Debug, Clone, Default)]
pub struct SearchParams {
    /// Embedding of the query prompt.
    pub query_embedding: Vec<f32>,
    /// Maximum number of results to return.
    pub limit: usize,
    /// Keep only chunks whose path starts with one of these prefixes (if any).
    pub starts_with: Vec<String>,
    /// Keep only chunks whose path ends with one of these suffixes (if any).
    pub ends_with: Vec<String>,
}

/// Outcome of an upload: how many chunk nodes were (re)indexed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UploadOutcome {
    /// Number of chunk nodes written.
    pub node_count: usize,
}

/// Persistence and search over workspaces.
#[async_trait]
pub trait WorkspaceStore: Send + Sync {
    /// Creates a new workspace for `working_dir` and returns its id.
    async fn create_workspace(&self, working_dir: &str) -> Result<String, ServerError>;

    /// Lists all workspaces.
    async fn list_workspaces(&self) -> Result<Vec<WorkspaceInfo>, ServerError>;

    /// Returns metadata for one workspace, or `None` if it does not exist.
    async fn get_workspace(&self, id: &str) -> Result<Option<WorkspaceInfo>, ServerError>;

    /// Deletes a workspace and all of its files and chunks.
    async fn delete_workspace(&self, id: &str) -> Result<(), ServerError>;

    /// Lists the files (with hashes) tracked in a workspace.
    async fn list_files(&self, id: &str) -> Result<Vec<FileHash>, ServerError>;

    /// Replaces the chunks for the given files in a workspace.
    ///
    /// All previously stored chunks for the affected file paths are removed and
    /// replaced by `chunks`. `files` records the post-upload file hashes.
    async fn upload(
        &self,
        id: &str,
        files: Vec<FileHash>,
        chunks: Vec<StoredChunk>,
    ) -> Result<UploadOutcome, ServerError>;

    /// Removes the given file paths (and their chunks) from a workspace.
    async fn delete_files(&self, id: &str, paths: &[String]) -> Result<(), ServerError>;

    /// Returns the most relevant chunks in a workspace for the query embedding.
    async fn search(&self, id: &str, params: SearchParams)
        -> Result<Vec<ScoredChunk>, ServerError>;
}

/// Cosine similarity between two vectors. Returns 0 for empty/zero vectors or
/// mismatched dimensions.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let mut dot = 0f32;
    let mut norm_a = 0f32;
    let mut norm_b = 0f32;
    for (x, y) in a.iter().zip(b.iter()) {
        dot += x * y;
        norm_a += x * x;
        norm_b += y * y;
    }
    let denom = norm_a.sqrt() * norm_b.sqrt();
    if denom == 0.0 { 0.0 } else { dot / denom }
}

/// On-disk snapshot of all workspaces.
#[derive(Debug, Default, Serialize, Deserialize)]
struct Snapshot {
    workspaces: HashMap<String, Workspace>,
}

/// In-memory representation of a single workspace.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Workspace {
    info: WorkspaceInfo,
    files: HashMap<String, FileHash>,
    chunks: Vec<StoredChunk>,
}

/// In-memory [`WorkspaceStore`] with optional JSON snapshot persistence.
///
/// When constructed with a snapshot path, the full state is flushed to disk
/// after every mutation and reloaded on startup. This keeps paid-for embeddings
/// durable across restarts without requiring an external database.
pub struct InMemoryWorkspaceStore {
    state: RwLock<Snapshot>,
    snapshot_path: Option<PathBuf>,
}

impl InMemoryWorkspaceStore {
    /// Creates an empty, non-persistent store (used in tests).
    pub fn in_memory() -> Self {
        Self { state: RwLock::new(Snapshot::default()), snapshot_path: None }
    }

    /// Opens a store persisted at `path`, loading any existing snapshot.
    ///
    /// # Errors
    /// Returns [`ServerError::Store`] if an existing snapshot cannot be read or
    /// parsed.
    pub async fn open(path: impl Into<PathBuf>) -> Result<Self, ServerError> {
        let path = path.into();
        let state = if tokio::fs::try_exists(&path).await.unwrap_or(false) {
            let bytes = tokio::fs::read(&path).await.map_err(|error| ServerError::Store {
                message: format!("failed to read snapshot {}: {error}", path.display()),
            })?;
            serde_json::from_slice(&bytes).map_err(|error| ServerError::Store {
                message: format!("failed to parse snapshot {}: {error}", path.display()),
            })?
        } else {
            Snapshot::default()
        };
        Ok(Self { state: RwLock::new(state), snapshot_path: Some(path) })
    }

    /// Flushes the current state to disk if persistence is configured.
    async fn persist(&self, snapshot: &Snapshot) -> Result<(), ServerError> {
        let Some(path) = &self.snapshot_path else {
            return Ok(());
        };
        let bytes = serde_json::to_vec(snapshot).map_err(|error| ServerError::Store {
            message: format!("failed to serialize snapshot: {error}"),
        })?;
        // Write to a temp file then rename for atomic, crash-safe replacement.
        let tmp = path.with_extension("tmp");
        tokio::fs::write(&tmp, &bytes).await.map_err(|error| ServerError::Store {
            message: format!("failed to write snapshot {}: {error}", tmp.display()),
        })?;
        tokio::fs::rename(&tmp, path).await.map_err(|error| ServerError::Store {
            message: format!("failed to commit snapshot {}: {error}", path.display()),
        })?;
        Ok(())
    }
}

#[async_trait]
impl WorkspaceStore for InMemoryWorkspaceStore {
    async fn create_workspace(&self, working_dir: &str) -> Result<String, ServerError> {
        let mut state = self.state.write().await;

        // Reuse an existing workspace for the same working dir so repeated
        // client sessions converge on one index instead of accumulating.
        if let Some(existing) = state.workspaces.values().find(|w| w.info.working_dir == working_dir)
        {
            return Ok(existing.info.id.clone());
        }

        let id = uuid::Uuid::new_v4().to_string();
        let workspace = Workspace {
            info: WorkspaceInfo {
                id: id.clone(),
                working_dir: working_dir.to_string(),
                created_at: Utc::now(),
                last_updated: None,
            },
            files: HashMap::new(),
            chunks: Vec::new(),
        };
        state.workspaces.insert(id.clone(), workspace);
        self.persist(&state).await?;
        Ok(id)
    }

    async fn list_workspaces(&self) -> Result<Vec<WorkspaceInfo>, ServerError> {
        let state = self.state.read().await;
        Ok(state.workspaces.values().map(|w| w.info.clone()).collect())
    }

    async fn get_workspace(&self, id: &str) -> Result<Option<WorkspaceInfo>, ServerError> {
        let state = self.state.read().await;
        Ok(state.workspaces.get(id).map(|w| w.info.clone()))
    }

    async fn delete_workspace(&self, id: &str) -> Result<(), ServerError> {
        let mut state = self.state.write().await;
        state.workspaces.remove(id);
        self.persist(&state).await?;
        Ok(())
    }

    async fn list_files(&self, id: &str) -> Result<Vec<FileHash>, ServerError> {
        let state = self.state.read().await;
        Ok(state
            .workspaces
            .get(id)
            .map(|w| w.files.values().cloned().collect())
            .unwrap_or_default())
    }

    async fn upload(
        &self,
        id: &str,
        files: Vec<FileHash>,
        chunks: Vec<StoredChunk>,
    ) -> Result<UploadOutcome, ServerError> {
        let mut state = self.state.write().await;
        let workspace = state.workspaces.get_mut(id).ok_or_else(|| ServerError::Store {
            message: format!("workspace {id} not found"),
        })?;

        // Replace chunks belonging to the uploaded paths.
        let uploaded_paths: std::collections::HashSet<&str> =
            files.iter().map(|f| f.path.as_str()).collect();
        workspace.chunks.retain(|c| !uploaded_paths.contains(c.path.as_str()));

        let node_count = chunks.len();
        workspace.chunks.extend(chunks);
        for file in files {
            workspace.files.insert(file.path.clone(), file);
        }
        workspace.info.last_updated = Some(Utc::now());

        self.persist(&state).await?;
        Ok(UploadOutcome { node_count })
    }

    async fn delete_files(&self, id: &str, paths: &[String]) -> Result<(), ServerError> {
        if paths.is_empty() {
            return Ok(());
        }
        let mut state = self.state.write().await;
        if let Some(workspace) = state.workspaces.get_mut(id) {
            let drop: std::collections::HashSet<&str> = paths.iter().map(|p| p.as_str()).collect();
            workspace.chunks.retain(|c| !drop.contains(c.path.as_str()));
            workspace.files.retain(|path, _| !drop.contains(path.as_str()));
            workspace.info.last_updated = Some(Utc::now());
            self.persist(&state).await?;
        }
        Ok(())
    }

    async fn search(
        &self,
        id: &str,
        params: SearchParams,
    ) -> Result<Vec<ScoredChunk>, ServerError> {
        let state = self.state.read().await;
        let Some(workspace) = state.workspaces.get(id) else {
            return Ok(Vec::new());
        };

        let mut scored: Vec<ScoredChunk> = workspace
            .chunks
            .iter()
            .filter(|chunk| path_matches(&chunk.path, &params.starts_with, &params.ends_with))
            .map(|chunk| {
                let relevance = cosine_similarity(&params.query_embedding, &chunk.embedding);
                ScoredChunk { chunk: chunk.clone(), relevance, distance: 1.0 - relevance }
            })
            .collect();

        scored.sort_by(|a, b| {
            b.relevance.partial_cmp(&a.relevance).unwrap_or(std::cmp::Ordering::Equal)
        });
        scored.truncate(params.limit.max(1));
        Ok(scored)
    }
}

/// Returns whether `path` satisfies the optional prefix/suffix filters. An
/// empty filter list means "no constraint".
fn path_matches(path: &str, starts_with: &[String], ends_with: &[String]) -> bool {
    let starts_ok = starts_with.is_empty() || starts_with.iter().any(|p| path.starts_with(p));
    let ends_ok = ends_with.is_empty() || ends_with.iter().any(|s| path.ends_with(s));
    starts_ok && ends_ok
}

/// Convenience alias for a shared, dynamically-dispatched workspace store.
pub type SharedWorkspaceStore = Arc<dyn WorkspaceStore>;

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    fn chunk(node_id: &str, path: &str, embedding: Vec<f32>) -> StoredChunk {
        StoredChunk {
            node_id: node_id.to_string(),
            path: path.to_string(),
            content: format!("content of {node_id}"),
            start_line: 1,
            end_line: 10,
            embedding,
        }
    }

    #[tokio::test]
    async fn test_create_and_get_workspace() {
        let store = InMemoryWorkspaceStore::in_memory();
        let id = store.create_workspace("/repo").await.unwrap();
        let actual = store.get_workspace(&id).await.unwrap().unwrap();
        assert_eq!(actual.working_dir, "/repo");
    }

    #[tokio::test]
    async fn test_create_workspace_is_idempotent_per_dir() {
        let store = InMemoryWorkspaceStore::in_memory();
        let first = store.create_workspace("/repo").await.unwrap();
        let second = store.create_workspace("/repo").await.unwrap();
        assert_eq!(first, second);
    }

    #[tokio::test]
    async fn test_upload_then_list_files() {
        let store = InMemoryWorkspaceStore::in_memory();
        let id = store.create_workspace("/repo").await.unwrap();
        let files = vec![FileHash { path: "a.rs".to_string(), hash: "h1".to_string() }];
        let chunks = vec![chunk("n1", "a.rs", vec![1.0, 0.0])];

        let outcome = store.upload(&id, files.clone(), chunks).await.unwrap();
        assert_eq!(outcome, UploadOutcome { node_count: 1 });

        let actual = store.list_files(&id).await.unwrap();
        assert_eq!(actual, files);
    }

    #[tokio::test]
    async fn test_upload_replaces_chunks_for_same_path() {
        let store = InMemoryWorkspaceStore::in_memory();
        let id = store.create_workspace("/repo").await.unwrap();
        store
            .upload(
                &id,
                vec![FileHash { path: "a.rs".to_string(), hash: "h1".to_string() }],
                vec![chunk("old", "a.rs", vec![1.0, 0.0])],
            )
            .await
            .unwrap();
        store
            .upload(
                &id,
                vec![FileHash { path: "a.rs".to_string(), hash: "h2".to_string() }],
                vec![chunk("new", "a.rs", vec![0.0, 1.0])],
            )
            .await
            .unwrap();

        let results = store
            .search(
                &id,
                SearchParams { query_embedding: vec![0.0, 1.0], limit: 10, ..Default::default() },
            )
            .await
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].chunk.node_id, "new");
    }

    #[tokio::test]
    async fn test_search_ranks_by_cosine_similarity() {
        let store = InMemoryWorkspaceStore::in_memory();
        let id = store.create_workspace("/repo").await.unwrap();
        store
            .upload(
                &id,
                vec![
                    FileHash { path: "a.rs".to_string(), hash: "h".to_string() },
                    FileHash { path: "b.rs".to_string(), hash: "h".to_string() },
                ],
                vec![
                    chunk("far", "a.rs", vec![0.0, 1.0]),
                    chunk("near", "b.rs", vec![1.0, 0.0]),
                ],
            )
            .await
            .unwrap();

        let results = store
            .search(
                &id,
                SearchParams { query_embedding: vec![1.0, 0.0], limit: 10, ..Default::default() },
            )
            .await
            .unwrap();
        assert_eq!(results[0].chunk.node_id, "near");
        assert!(results[0].relevance > results[1].relevance);
    }

    #[tokio::test]
    async fn test_search_respects_path_filters() {
        let store = InMemoryWorkspaceStore::in_memory();
        let id = store.create_workspace("/repo").await.unwrap();
        store
            .upload(
                &id,
                vec![
                    FileHash { path: "src/a.rs".to_string(), hash: "h".to_string() },
                    FileHash { path: "docs/b.md".to_string(), hash: "h".to_string() },
                ],
                vec![
                    chunk("rs", "src/a.rs", vec![1.0, 0.0]),
                    chunk("md", "docs/b.md", vec![1.0, 0.0]),
                ],
            )
            .await
            .unwrap();

        let results = store
            .search(
                &id,
                SearchParams {
                    query_embedding: vec![1.0, 0.0],
                    limit: 10,
                    ends_with: vec![".rs".to_string()],
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].chunk.node_id, "rs");
    }

    #[tokio::test]
    async fn test_delete_files_removes_chunks() {
        let store = InMemoryWorkspaceStore::in_memory();
        let id = store.create_workspace("/repo").await.unwrap();
        store
            .upload(
                &id,
                vec![FileHash { path: "a.rs".to_string(), hash: "h".to_string() }],
                vec![chunk("n", "a.rs", vec![1.0, 0.0])],
            )
            .await
            .unwrap();

        store.delete_files(&id, &["a.rs".to_string()]).await.unwrap();
        assert_eq!(store.list_files(&id).await.unwrap(), Vec::new());
    }

    #[tokio::test]
    async fn test_persistence_round_trip() {
        let dir = std::env::temp_dir().join(format!("forge-store-{}", uuid::Uuid::new_v4()));
        let path = dir.join("snapshot.json");
        tokio::fs::create_dir_all(&dir).await.unwrap();

        let id = {
            let store = InMemoryWorkspaceStore::open(&path).await.unwrap();
            let id = store.create_workspace("/repo").await.unwrap();
            store
                .upload(
                    &id,
                    vec![FileHash { path: "a.rs".to_string(), hash: "h".to_string() }],
                    vec![chunk("n", "a.rs", vec![1.0, 0.0])],
                )
                .await
                .unwrap();
            id
        };

        let reopened = InMemoryWorkspaceStore::open(&path).await.unwrap();
        let actual = reopened.list_files(&id).await.unwrap();
        assert_eq!(actual, vec![FileHash { path: "a.rs".to_string(), hash: "h".to_string() }]);

        tokio::fs::remove_dir_all(&dir).await.ok();
    }

    #[test]
    fn test_cosine_similarity_orthogonal_is_zero() {
        let actual = cosine_similarity(&[1.0, 0.0], &[0.0, 1.0]);
        assert_eq!(actual, 0.0);
    }

    #[test]
    fn test_cosine_similarity_identical_is_one() {
        let actual = cosine_similarity(&[1.0, 2.0, 3.0], &[1.0, 2.0, 3.0]);
        assert!((actual - 1.0).abs() < 1e-6);
    }
}
