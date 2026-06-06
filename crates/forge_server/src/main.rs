//! Binary entrypoint for the Mnethos server.
//!
//! Runs two servers concurrently:
//! - the **auth REST** API (`/auth/user`, `/auth/usage`, `/health`) on
//!   [`REST_ADDRESS_ENV`];
//! - the **context-engine gRPC** service (`mnethos.v1.MnethosService`) on
//!   [`GRPC_ADDRESS_ENV`].
//!
//! In production a TLS-terminating reverse proxy fronts both on a single public
//! hostname (e.g. `api.mnethos.com`), routing gRPC (HTTP/2, `application/grpc`)
//! to the gRPC port and the REST paths to the REST port.

use std::sync::Arc;

use forge_server::proto::mnethos::mnethos_service_server::MnethosServiceServer;
use forge_server::workspace::InMemoryWorkspaceStore;
use forge_server::{
    AiGatewayEmbedder, ContextEngineService, InMemoryUserStore, Server, SharedWorkspaceStore,
};

/// Listen address for the auth REST server (default `0.0.0.0:8080`).
const REST_ADDRESS_ENV: &str = "MNETHOS_REST_ADDRESS";
/// Default REST listen address.
const DEFAULT_REST_ADDRESS: &str = "0.0.0.0:8080";

/// Listen address for the context-engine gRPC server (default `0.0.0.0:50051`).
const GRPC_ADDRESS_ENV: &str = "MNETHOS_GRPC_ADDRESS";
/// Default gRPC listen address.
const DEFAULT_GRPC_ADDRESS: &str = "0.0.0.0:50051";

/// Endpoint of the ai-gateway embedding service (default `http://ai-gateway:50054`).
const AI_GATEWAY_ENV: &str = "MNETHOS_AI_GATEWAY_ADDR";
/// Default ai-gateway endpoint (matches the `pag-prod` compose network).
const DEFAULT_AI_GATEWAY: &str = "http://ai-gateway:50054";

/// Filesystem path for the workspace snapshot (default `/data/workspaces.json`).
const SNAPSHOT_ENV: &str = "MNETHOS_SNAPSHOT_PATH";
/// Default snapshot path (mount a volume here to persist embeddings).
const DEFAULT_SNAPSHOT: &str = "/data/workspaces.json";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let rest_address =
        std::env::var(REST_ADDRESS_ENV).unwrap_or_else(|_| DEFAULT_REST_ADDRESS.to_string());
    let grpc_address =
        std::env::var(GRPC_ADDRESS_ENV).unwrap_or_else(|_| DEFAULT_GRPC_ADDRESS.to_string());
    let ai_gateway =
        std::env::var(AI_GATEWAY_ENV).unwrap_or_else(|_| DEFAULT_AI_GATEWAY.to_string());
    let snapshot = std::env::var(SNAPSHOT_ENV).unwrap_or_else(|_| DEFAULT_SNAPSHOT.to_string());

    // ---- Auth REST server (blocking tiny_http loop on its own thread). ----
    let user_store = Arc::new(InMemoryUserStore::demo());
    let rest_server = Server::new(user_store);
    let rest_handle = tokio::task::spawn_blocking(move || rest_server.run(&rest_address));

    // ---- Context-engine gRPC server. ----
    let store: SharedWorkspaceStore = Arc::new(InMemoryWorkspaceStore::open(&snapshot).await?);
    let embedder = Arc::new(AiGatewayEmbedder::new(ai_gateway.clone()));
    let service = ContextEngineService::new(embedder, store);

    let grpc_addr = grpc_address
        .parse()
        .map_err(|e| anyhow::anyhow!("invalid gRPC address `{grpc_address}`: {e}"))?;

    tracing::info!(
        grpc = %grpc_address,
        ai_gateway = %ai_gateway,
        snapshot = %snapshot,
        "Mnethos context-engine gRPC listening"
    );

    let grpc_handle = tokio::spawn(async move {
        tonic::transport::Server::builder()
            .add_service(MnethosServiceServer::new(service))
            .serve(grpc_addr)
            .await
    });

    // If either server exits, propagate the result and shut down.
    tokio::select! {
        result = rest_handle => {
            result.map_err(|e| anyhow::anyhow!("REST task panicked: {e}"))??;
        }
        result = grpc_handle => {
            result.map_err(|e| anyhow::anyhow!("gRPC task panicked: {e}"))??;
        }
    }

    Ok(())
}
