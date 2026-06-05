//! The Mnethos memory layer, kept entirely OUT of core forgecode crates.
//!
//! Lifecycle (gated by the single `MNETHOS_MEMORY` env flag; no-op otherwise):
//!   * Start → [`retrieval`]: model-generated anchors → fast `Retrieve` over gRPC
//!     → inject `<recalled_project_memory>` into the system prompt.
//!   * End → [`observer`] (snapshot dump) + [`consolidation`] (extract episodes →
//!     write over gRPC).
//!
//! Connection config lives in a crate-owned `memory.json` (see [`config`]) — no
//! coupling to core's provider/credential system. Wire the hooks into the agent
//! lifecycle from `forge_api` via [`start_hook`] / [`end_hook`].

use std::sync::Arc;

use forge_app::{EnvironmentInfra, LifecycleHooks, Services};
use forge_domain::{EndPayload, EventData, EventHandle, EventHandleExt, StartPayload};

mod client;
mod config;
mod consolidation;
mod domain;
mod observer;
mod retrieval;
mod retry;
mod session;

/// Generated gRPC stubs for the memory service (ai_working_memory.memorywrite.v1).
mod proto {
    tonic::include_proto!("ai_working_memory.memorywrite.v1");
}

pub use config::MemoryConfig;

/// Bound shared by both hooks: a Services impl that also exposes the environment
/// (cwd) — which is exactly what core's concrete infra provides. The hooks use
/// only existing core traits (read-only): `chat_agent` (AgentService blanket
/// impl) and `get_environment().cwd`.
pub trait MemoryServices:
    Services + EnvironmentInfra<Config = forge_config::ForgeConfig> + 'static
{
}
impl<T: Services + EnvironmentInfra<Config = forge_config::ForgeConfig> + 'static> MemoryServices
    for T
{
}

/// Start-of-turn hook: recalls relevant project memory and injects it into the
/// system prompt before the model runs. No-op unless `MNETHOS_MEMORY` is set and
/// `memory.json` is present.
pub fn start_hook<S: MemoryServices>(
    services: Arc<S>,
) -> Box<dyn EventHandle<EventData<StartPayload>>> {
    Box::new(retrieval::RetrievalHandler::new(services))
}

/// End-of-turn hook: dumps a consolidation snapshot, then extracts and writes
/// durable episodes to memory. No-op unless `MNETHOS_MEMORY` is set; the write
/// itself no-ops without `memory.json`.
pub fn end_hook<S: MemoryServices>(
    services: Arc<S>,
) -> Box<dyn EventHandle<EventData<EndPayload>>> {
    observer::MemoryObserver::new().and(consolidation::MemoryConsolidationHandler::new(services))
}

/// Provider that plugs the memory layer's Start/End hooks into `ForgeApp` via
/// the generic `LifecycleHooks` seam. Wire this in from `forge_api`:
/// `ForgeApp::new(services).with_lifecycle_hooks(forge_memory::lifecycle(services))`.
struct MemoryLifecycle<S> {
    services: Arc<S>,
}

impl<S: MemoryServices> LifecycleHooks for MemoryLifecycle<S> {
    fn on_start(&self) -> Option<Box<dyn EventHandle<EventData<StartPayload>>>> {
        Some(start_hook(self.services.clone()))
    }

    fn on_end(&self) -> Option<Box<dyn EventHandle<EventData<EndPayload>>>> {
        Some(end_hook(self.services.clone()))
    }
}

/// Builds the memory [`LifecycleHooks`] provider for `ForgeApp::with_lifecycle_hooks`.
pub fn lifecycle<S: MemoryServices>(services: Arc<S>) -> Arc<dyn LifecycleHooks> {
    Arc::new(MemoryLifecycle { services })
}
