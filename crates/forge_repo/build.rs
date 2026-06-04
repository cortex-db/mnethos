fn main() -> Result<(), Box<dyn std::error::Error>> {
    // forge.v1 — workspace/context-engine service (forgecode services).
    tonic_prost_build::compile_protos("proto/forge.proto")?;
    // ai_working_memory.memorywrite.v1 — Mnethos memory write client
    // (mirror of ai-working-memory/proto/memorywrite).
    tonic_prost_build::compile_protos("proto/memorywrite.proto")?;
    Ok(())
}
