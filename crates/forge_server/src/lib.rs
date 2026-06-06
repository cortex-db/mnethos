//! Mnethos reference server.
//!
//! This crate implements the minimal backend contract the Mnethos CLI depends
//! on: the `auth/user` and `auth/usage` REST endpoints consumed by
//! `crates/forge_services/src/auth.rs`. It is intentionally self-contained so
//! the client no longer requires an external hosted account service to
//! resolve account identity and usage.
//!
//! The crate exposes:
//! - [`dto`]: wire-compatible request/response types (`camelCase` JSON).
//! - [`store`]: the [`UserStore`] abstraction and an in-memory implementation.
//! - [`server`]: the HTTP [`Server`] (auth REST) and its pure routing core.
//! - [`chunk`], [`embedding`], [`workspace`], [`grpc`]: the semantic-search
//!   context engine (`mnethos.v1.MnethosService`) backed by the ai-gateway for
//!   embeddings and an embedded vector store for chunk search.

pub mod chunk;
pub mod dto;
pub mod embedding;
pub mod error;
pub mod grpc;
pub mod proto;
pub mod server;
pub mod store;
pub mod workspace;

pub use dto::{AuthProviderId, Plan, UsageInfo, User, UserUsage};
pub use embedding::{AiGatewayEmbedder, Embedder, EMBEDDING_DIMENSION, EMBEDDING_MODEL};
pub use error::ServerError;
pub use grpc::ContextEngineService;
pub use server::{HttpResponse, Server};
pub use store::{InMemoryUserStore, UserStore};
pub use workspace::{
    InMemoryWorkspaceStore, SharedWorkspaceStore, WorkspaceInfo, WorkspaceStore,
};
