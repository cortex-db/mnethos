use forge_domain::{EndPayload, EventData, EventHandle, StartPayload};

/// Generic extension point for injecting extra agent-lifecycle hooks from a
/// higher layer, WITHOUT this crate depending on that layer.
///
/// `ForgeApp::with_lifecycle_hooks` accepts an `Arc<dyn LifecycleHooks>`; during
/// each chat turn `ForgeApp` folds the returned handlers into its own Start/End
/// hook chains. Each method returns a FRESH boxed handler per turn (or `None` to
/// add nothing), so the provider can be held behind a shared `Arc` and reused.
///
/// This is deliberately feature-agnostic: it is how the out-of-core memory crate
/// (`forge_memory`) plugs retrieval (Start) and consolidation (End) into the run.
pub trait LifecycleHooks: Send + Sync {
    /// Handler to run at the start of a turn (before the model sees the context).
    fn on_start(&self) -> Option<Box<dyn EventHandle<EventData<StartPayload>>>> {
        None
    }

    /// Handler to run at the end of a turn (after the agent finishes).
    fn on_end(&self) -> Option<Box<dyn EventHandle<EventData<EndPayload>>>> {
        None
    }
}
