//! Generated gRPC stubs for the context-engine and embedding-gateway services.
//!
//! The modules here are produced at build time by `tonic-prost-build` from the
//! vendored `.proto` files under `proto/`. We re-export them under stable,
//! descriptive names so the rest of the crate does not reference the raw
//! `include_proto!` macro output.

/// `forge.v1` — the context-engine service implemented by this server.
pub mod forge {
    tonic::include_proto!("forge.v1");
}

/// `ai_gateway.v1` — the embedding gateway this server calls for embeddings.
pub mod ai_gateway {
    tonic::include_proto!("ai_gateway.v1");
}
