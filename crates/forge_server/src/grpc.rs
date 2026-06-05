//! `forge.v1.MnethosService` gRPC implementation — the context engine.
//!
//! This service is what the Mnethos CLI's `services_url` gRPC channel talks to.
//! It implements the nine RPCs the client actually calls (workspace lifecycle,
//! file upload/list/delete and semantic search); the remaining RPCs are handled
//! locally by the client and are reported as `unimplemented` here.
//!
//! Uploads are chunked and embedded via the ai-gateway, then persisted in the
//! [`WorkspaceStore`]. Search embeds the query and ranks stored chunks by cosine
//! similarity.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use tonic::{Request, Response, Status};

use crate::chunk::{chunk_file, ChunkConfig};
use crate::embedding::Embedder;
use crate::proto::forge::mnethos_service_server::MnethosService;
use crate::proto::forge::node_data::Kind as NodeKindData;
use crate::proto::forge::{
    BuildTextPatchRequest, BuildTextPatchResponse, ChunkFilesRequest, ChunkFilesResponse,
    CreateApiKeyRequest, CreateApiKeyResponse, CreateWorkspaceRequest, CreateWorkspaceResponse,
    DeleteFilesRequest, DeleteFilesResponse, DeleteWorkspaceRequest, DeleteWorkspaceResponse,
    FileChunk, FileRef, FileRefNode, FuzzySearchRequest, FuzzySearchResponse,
    GetWorkspaceInfoRequest, GetWorkspaceInfoResponse, HealthCheckRequest, HealthCheckResponse,
    ListFilesRequest, ListFilesResponse, ListWorkspacesRequest, ListWorkspacesResponse, Node,
    NodeData, NodeId, QueryItem, QueryResult, SearchRequest, SearchResponse, SelectSkillRequest,
    SelectSkillResponse, UploadFilesRequest, UploadFilesResponse, UploadResult, UserId,
    ValidateFilesRequest, ValidateFilesResponse, Workspace, WorkspaceId,
};
use crate::workspace::{FileHash, SearchParams, SharedWorkspaceStore, StoredChunk, WorkspaceInfo};

/// Default search result count when the client supplies neither `limit` nor
/// `top_k`.
const DEFAULT_SEARCH_LIMIT: usize = 20;

/// Implementation of the context-engine gRPC service.
///
/// Generic over the [`Embedder`] so tests can inject a deterministic fake while
/// production wires the ai-gateway client.
pub struct ContextEngineService<E> {
    embedder: Arc<E>,
    store: SharedWorkspaceStore,
}

impl<E> ContextEngineService<E> {
    /// Creates a service from an embedder and a workspace store.
    pub fn new(embedder: Arc<E>, store: SharedWorkspaceStore) -> Self {
        Self { embedder, store }
    }
}

impl<E: Embedder + 'static> ContextEngineService<E> {
    /// Extracts a workspace id from an optional proto [`WorkspaceId`].
    fn workspace_id(id: Option<WorkspaceId>) -> Result<String, Status> {
        id.map(|w| w.id).filter(|s| !s.is_empty()).ok_or_else(|| {
            Status::invalid_argument("workspace_id is required")
        })
    }
}

/// Converts a chrono timestamp into the protobuf well-known `Timestamp`.
fn to_timestamp(dt: DateTime<Utc>) -> prost_types::Timestamp {
    prost_types::Timestamp { seconds: dt.timestamp(), nanos: dt.timestamp_subsec_nanos() as i32 }
}

/// Builds the proto [`Workspace`] message from stored workspace metadata.
fn workspace_proto(info: &WorkspaceInfo) -> Workspace {
    Workspace {
        workspace_id: Some(WorkspaceId { id: info.id.clone() }),
        working_dir: info.working_dir.clone(),
        node_count: None,
        relation_count: None,
        last_updated: info.last_updated.map(to_timestamp),
        min_chunk_size: 0,
        max_chunk_size: 0,
        created_at: Some(to_timestamp(info.created_at)),
    }
}

#[tonic::async_trait]
impl<E: Embedder + 'static> MnethosService for ContextEngineService<E> {
    async fn create_api_key(
        &self,
        _request: Request<CreateApiKeyRequest>,
    ) -> Result<Response<CreateApiKeyResponse>, Status> {
        // Single-tenant self-hosted deployment: mint an opaque key. The network
        // boundary (reverse proxy / private network) is the trust boundary.
        let user_id = uuid::Uuid::new_v4().to_string();
        let key = format!("mnethos-{}", uuid::Uuid::new_v4());
        Ok(Response::new(CreateApiKeyResponse { user_id: Some(UserId { id: user_id }), key }))
    }

    async fn create_workspace(
        &self,
        request: Request<CreateWorkspaceRequest>,
    ) -> Result<Response<CreateWorkspaceResponse>, Status> {
        let definition = request
            .into_inner()
            .workspace
            .ok_or_else(|| Status::invalid_argument("workspace definition is required"))?;

        let id = self
            .store
            .create_workspace(&definition.working_dir)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let info = self
            .store
            .get_workspace(&id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?
            .ok_or_else(|| Status::internal("workspace vanished after creation"))?;

        Ok(Response::new(CreateWorkspaceResponse { workspace: Some(workspace_proto(&info)) }))
    }

    async fn list_workspaces(
        &self,
        _request: Request<ListWorkspacesRequest>,
    ) -> Result<Response<ListWorkspacesResponse>, Status> {
        let workspaces = self
            .store
            .list_workspaces()
            .await
            .map_err(|e| Status::internal(e.to_string()))?
            .iter()
            .map(workspace_proto)
            .collect();
        Ok(Response::new(ListWorkspacesResponse { workspaces }))
    }

    async fn get_workspace_info(
        &self,
        request: Request<GetWorkspaceInfoRequest>,
    ) -> Result<Response<GetWorkspaceInfoResponse>, Status> {
        let id = Self::workspace_id(request.into_inner().workspace_id)?;
        let workspace = self
            .store
            .get_workspace(&id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?
            .map(|info| workspace_proto(&info));
        Ok(Response::new(GetWorkspaceInfoResponse { workspace }))
    }

    async fn delete_workspace(
        &self,
        request: Request<DeleteWorkspaceRequest>,
    ) -> Result<Response<DeleteWorkspaceResponse>, Status> {
        let id = Self::workspace_id(request.into_inner().workspace_id)?;
        self.store.delete_workspace(&id).await.map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(DeleteWorkspaceResponse {
            workspace_id: Some(WorkspaceId { id }),
        }))
    }

    async fn list_files(
        &self,
        request: Request<ListFilesRequest>,
    ) -> Result<Response<ListFilesResponse>, Status> {
        let id = Self::workspace_id(request.into_inner().workspace_id)?;
        let files = self
            .store
            .list_files(&id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?
            .into_iter()
            .map(|fh| FileRefNode {
                node_id: Some(NodeId { id: fh.path.clone() }),
                hash: fh.hash.clone(),
                git: None,
                data: Some(FileRef { path: fh.path, file_hash: fh.hash }),
            })
            .collect();
        Ok(Response::new(ListFilesResponse { files }))
    }

    async fn upload_files(
        &self,
        request: Request<UploadFilesRequest>,
    ) -> Result<Response<UploadFilesResponse>, Status> {
        let message = request.into_inner();
        let id = Self::workspace_id(message.workspace_id)?;
        let content = message
            .content
            .ok_or_else(|| Status::invalid_argument("upload content is required"))?;

        // Chunk every uploaded file and remember each chunk's file hash.
        let config = ChunkConfig::default();
        let mut chunk_texts: Vec<String> = Vec::new();
        let mut pending: Vec<(String, u32, u32, String)> = Vec::new();
        let mut files: Vec<FileHash> = Vec::new();

        for file in content.files {
            let hash = content_hash(&file.content);
            files.push(FileHash { path: file.path.clone(), hash });
            for chunk in chunk_file(&file.path, &file.content, config) {
                pending.push((chunk.path, chunk.start_line, chunk.end_line, chunk.content.clone()));
                chunk_texts.push(chunk.content);
            }
        }

        let embeddings = if chunk_texts.is_empty() {
            Vec::new()
        } else {
            self.embedder
                .embed(&chunk_texts)
                .await
                .map_err(|e| Status::internal(e.to_string()))?
        };

        let stored: Vec<StoredChunk> = pending
            .into_iter()
            .zip(embeddings)
            .map(|((path, start_line, end_line, text), embedding)| StoredChunk {
                node_id: uuid::Uuid::new_v4().to_string(),
                path,
                content: text,
                start_line,
                end_line,
                embedding,
            })
            .collect();

        let outcome = self
            .store
            .upload(&id, files, stored.clone())
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let node_ids = stored.into_iter().map(|c| c.node_id).collect();
        let _ = outcome;
        Ok(Response::new(UploadFilesResponse {
            result: Some(UploadResult { node_ids, relations: Vec::new() }),
        }))
    }

    async fn delete_files(
        &self,
        request: Request<DeleteFilesRequest>,
    ) -> Result<Response<DeleteFilesResponse>, Status> {
        let message = request.into_inner();
        let id = Self::workspace_id(message.workspace_id)?;
        let deleted = message.file_paths.len() as u32;
        self.store
            .delete_files(&id, &message.file_paths)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(DeleteFilesResponse {
            deleted_nodes: deleted,
            deleted_relations: 0,
        }))
    }

    async fn search(
        &self,
        request: Request<SearchRequest>,
    ) -> Result<Response<SearchResponse>, Status> {
        let message = request.into_inner();
        let id = Self::workspace_id(message.workspace_id)?;
        let query = message
            .query
            .ok_or_else(|| Status::invalid_argument("query is required"))?;

        let prompt = query.prompt.unwrap_or_default();
        if prompt.trim().is_empty() {
            return Ok(Response::new(SearchResponse {
                result: Some(QueryResult { data: Vec::new() }),
            }));
        }

        let limit = query
            .limit
            .or(query.top_k)
            .map(|l| l as usize)
            .filter(|l| *l > 0)
            .unwrap_or(DEFAULT_SEARCH_LIMIT);

        let embedding = self
            .embedder
            .embed(std::slice::from_ref(&prompt))
            .await
            .map_err(|e| Status::internal(e.to_string()))?
            .into_iter()
            .next()
            .ok_or_else(|| Status::internal("embedder returned no vector for query"))?;

        let scored = self
            .store
            .search(
                &id,
                SearchParams {
                    query_embedding: embedding,
                    limit,
                    starts_with: query.starts_with,
                    ends_with: query.ends_with,
                },
            )
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let data = scored
            .into_iter()
            .enumerate()
            .map(|(rank, scored)| QueryItem {
                node: Some(Node {
                    node_id: Some(NodeId { id: scored.chunk.node_id }),
                    workspace_id: Some(WorkspaceId { id: id.clone() }),
                    hash: String::new(),
                    git: None,
                    data: Some(NodeData {
                        kind: Some(NodeKindData::FileChunk(FileChunk {
                            path: scored.chunk.path,
                            content: scored.chunk.content,
                            start_line: scored.chunk.start_line,
                            end_line: scored.chunk.end_line,
                        })),
                    }),
                }),
                distance: Some(scored.distance),
                rank: Some(rank as u64),
                relevance: Some(scored.relevance),
            })
            .collect();

        Ok(Response::new(SearchResponse { result: Some(QueryResult { data }) }))
    }

    async fn health_check(
        &self,
        _request: Request<HealthCheckRequest>,
    ) -> Result<Response<HealthCheckResponse>, Status> {
        Ok(Response::new(HealthCheckResponse { status: "ok".to_string() }))
    }

    // ---- RPCs handled locally by the client; not served here. ----

    async fn chunk_files(
        &self,
        _request: Request<ChunkFilesRequest>,
    ) -> Result<Response<ChunkFilesResponse>, Status> {
        Err(Status::unimplemented("ChunkFiles is handled client-side"))
    }

    async fn validate_files(
        &self,
        _request: Request<ValidateFilesRequest>,
    ) -> Result<Response<ValidateFilesResponse>, Status> {
        Err(Status::unimplemented("ValidateFiles is handled client-side"))
    }

    async fn select_skill(
        &self,
        _request: Request<SelectSkillRequest>,
    ) -> Result<Response<SelectSkillResponse>, Status> {
        Err(Status::unimplemented("SelectSkill is handled client-side"))
    }

    async fn fuzzy_search(
        &self,
        _request: Request<FuzzySearchRequest>,
    ) -> Result<Response<FuzzySearchResponse>, Status> {
        Err(Status::unimplemented("FuzzySearch is handled client-side"))
    }

    async fn build_text_patch(
        &self,
        _request: Request<BuildTextPatchRequest>,
    ) -> Result<Response<BuildTextPatchResponse>, Status> {
        Err(Status::unimplemented("BuildTextPatch is handled client-side"))
    }
}

/// Computes a stable content hash used to track file changes.
fn content_hash(content: &str) -> String {
    // FNV-1a 64-bit: small, dependency-free, and only needs to detect change.
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in content.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::embedding::tests::FakeEmbedder;
    use crate::proto::forge::{FileUploadContent, Query};
    use crate::workspace::InMemoryWorkspaceStore;

    fn fixture() -> ContextEngineService<FakeEmbedder> {
        let embedder = Arc::new(FakeEmbedder::new(64));
        let store: SharedWorkspaceStore = Arc::new(InMemoryWorkspaceStore::in_memory());
        ContextEngineService::new(embedder, store)
    }

    async fn make_workspace(service: &ContextEngineService<FakeEmbedder>) -> String {
        let response = service
            .create_workspace(Request::new(CreateWorkspaceRequest {
                workspace: Some(crate::proto::forge::WorkspaceDefinition {
                    working_dir: "/repo".to_string(),
                    min_chunk_size: 0,
                    max_chunk_size: 0,
                }),
            }))
            .await
            .unwrap()
            .into_inner();
        response.workspace.unwrap().workspace_id.unwrap().id
    }

    #[tokio::test]
    async fn test_create_api_key_returns_key() {
        let actual = fixture()
            .create_api_key(Request::new(CreateApiKeyRequest { user_id: None }))
            .await
            .unwrap()
            .into_inner();
        assert!(actual.key.starts_with("mnethos-"));
        assert!(actual.user_id.is_some());
    }

    #[tokio::test]
    async fn test_upload_then_search_finds_relevant_chunk() {
        let service = fixture();
        let id = make_workspace(&service).await;

        service
            .upload_files(Request::new(UploadFilesRequest {
                workspace_id: Some(WorkspaceId { id: id.clone() }),
                content: Some(FileUploadContent {
                    files: vec![
                        crate::proto::forge::File {
                            path: "auth.rs".to_string(),
                            content: "fn authenticate_user(token: &str) -> bool { true }"
                                .to_string(),
                        },
                        crate::proto::forge::File {
                            path: "math.rs".to_string(),
                            content: "fn add(a: i32, b: i32) -> i32 { a + b }".to_string(),
                        },
                    ],
                    git: None,
                }),
            }))
            .await
            .unwrap();

        let response = service
            .search(Request::new(SearchRequest {
                workspace_id: Some(WorkspaceId { id: id.clone() }),
                query: Some(Query {
                    prompt: Some("fn authenticate_user(token: &str) -> bool { true }".to_string()),
                    limit: Some(5),
                    ..Default::default()
                }),
            }))
            .await
            .unwrap()
            .into_inner();

        let data = response.result.unwrap().data;
        assert!(!data.is_empty());
        // The exact-match chunk must rank first.
        let top = data.first().unwrap().node.as_ref().unwrap().data.as_ref().unwrap();
        match top.kind.as_ref().unwrap() {
            NodeKindData::FileChunk(chunk) => assert_eq!(chunk.path, "auth.rs"),
            _ => panic!("expected a file chunk"),
        }
    }

    #[tokio::test]
    async fn test_list_files_after_upload() {
        let service = fixture();
        let id = make_workspace(&service).await;
        service
            .upload_files(Request::new(UploadFilesRequest {
                workspace_id: Some(WorkspaceId { id: id.clone() }),
                content: Some(FileUploadContent {
                    files: vec![crate::proto::forge::File {
                        path: "a.rs".to_string(),
                        content: "fn main() {}".to_string(),
                    }],
                    git: None,
                }),
            }))
            .await
            .unwrap();

        let response = service
            .list_files(Request::new(ListFilesRequest {
                workspace_id: Some(WorkspaceId { id }),
            }))
            .await
            .unwrap()
            .into_inner();
        assert_eq!(response.files.len(), 1);
        assert_eq!(response.files[0].data.as_ref().unwrap().path, "a.rs");
    }

    #[tokio::test]
    async fn test_missing_workspace_id_is_invalid_argument() {
        let service = fixture();
        let status = service
            .list_files(Request::new(ListFilesRequest { workspace_id: None }))
            .await
            .unwrap_err();
        assert_eq!(status.code(), tonic::Code::InvalidArgument);
    }

    #[tokio::test]
    async fn test_locally_handled_rpc_is_unimplemented() {
        let service = fixture();
        let status = service
            .fuzzy_search(Request::new(FuzzySearchRequest {
                needle: "x".to_string(),
                haystack: "y".to_string(),
                search_all: false,
            }))
            .await
            .unwrap_err();
        assert_eq!(status.code(), tonic::Code::Unimplemented);
    }
}
