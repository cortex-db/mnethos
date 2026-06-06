fn main() -> Result<(), Box<dyn std::error::Error>> {
    // mnethos.v1 — workspace/context-engine service (Mnethos services).
    tonic_prost_build::compile_protos("proto/forge.proto")?;
    // ai_working_memory.memorywrite.v1 — Mnethos long-term memory client.
    tonic_prost_build::compile_protos("proto/memorywrite.proto")?;
    Ok(())
}
