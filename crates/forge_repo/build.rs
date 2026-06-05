fn main() -> Result<(), Box<dyn std::error::Error>> {
    // forge.v1 — workspace/context-engine service (forgecode services).
    tonic_prost_build::compile_protos("proto/forge.proto")?;
    Ok(())
}
