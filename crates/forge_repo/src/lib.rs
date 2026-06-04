mod agent;
mod agent_definition;
mod context_engine;
mod conversation;
mod database;
mod forge_repo;
mod fs_snap;
mod fuzzy_search;
mod memory;
mod provider;
mod skill;
mod validation;

mod proto_generated {
    tonic::include_proto!("forge.v1");
}

mod memorywrite_proto {
    tonic::include_proto!("ai_working_memory.memorywrite.v1");
}

// Only expose forge_repo container
pub use forge_repo::*;
