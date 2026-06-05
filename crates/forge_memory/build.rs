fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ai_working_memory.memorywrite.v1 — Mnethos memory gRPC contract (mirror of
    // ai-working-memory/proto/memorywrite; client-side copy).
    tonic_prost_build::compile_protos("proto/memorywrite.proto")?;
    Ok(())
}
