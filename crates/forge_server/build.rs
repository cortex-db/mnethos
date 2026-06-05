fn main() -> Result<(), Box<dyn std::error::Error>> {
    // forge.v1 — the context-engine service the Mnethos CLI client speaks to.
    // We generate the gRPC *server* stubs here (the client lives in forge_repo).
    tonic_prost_build::compile_protos("proto/forge.proto")?;
    // ai_gateway.v1 — the embedding gateway. We generate the *client* stub to
    // call AIGateway.Embed for embeddings.
    tonic_prost_build::compile_protos("proto/aigateway.proto")?;
    Ok(())
}
