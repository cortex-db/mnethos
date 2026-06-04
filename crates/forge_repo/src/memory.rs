use anyhow::{Context, Result};
use async_trait::async_trait;
use forge_domain::{ApiKey, MemoryEpisode, MemoryWriteRepository, MemoryWriteResult};
use tonic::transport::{Channel, ClientTlsConfig};

use crate::memorywrite_proto::memory_write_client::MemoryWriteClient;
use crate::memorywrite_proto::{self as pb};

/// gRPC client for the ai-working-memory "memory" provider (MemoryWrite service).
///
/// Self-contained: builds its own lazy channel to the memory server URL (taken
/// from the "memory" provider credential), independent of the forgecode /
/// context-engine `services_url` channel. Auth: `Bearer <api_key>` in request
/// metadata — same scheme as [`ForgeContextEngineRepository`](crate).
#[derive(Default)]
pub struct ForgeMemoryRepository;

impl ForgeMemoryRepository {
    pub fn new() -> Self {
        Self
    }
}

/// Builds a lazily-connected channel to the memory server, enabling TLS for
/// https URLs (webpki roots, like the workspace gRPC client).
fn build_channel(server_url: &str) -> Result<Channel> {
    let endpoint = Channel::from_shared(server_url.to_string())
        .context("invalid memory server URL")?;
    let endpoint = if server_url.starts_with("https://") {
        endpoint
            .tls_config(ClientTlsConfig::new().with_webpki_roots())
            .context("failed to configure TLS for memory channel")?
    } else {
        endpoint
    };
    Ok(endpoint.connect_lazy())
}

#[async_trait]
impl MemoryWriteRepository for ForgeMemoryRepository {
    async fn create_episode(
        &self,
        server_url: &str,
        auth_token: &ApiKey,
        session_key: &str,
        episode: MemoryEpisode,
    ) -> Result<MemoryWriteResult> {
        let channel = build_channel(server_url)?;
        let mut client = MemoryWriteClient::new(channel);

        let mut request = tonic::Request::new(pb::CreateEpisodeRequest {
            session_key: session_key.to_string(),
            episode: Some(to_pb_episode(episode)),
        });
        request
            .metadata_mut()
            .insert("authorization", format!("Bearer {}", **auth_token).parse()?);

        let resp = client
            .create_episode(request)
            .await
            .context("memory CreateEpisode gRPC call failed")?
            .into_inner();

        Ok(MemoryWriteResult {
            episode_id: resp.episode_id,
            concept_count: resp.concept_count,
            anchor_count: resp.anchor_count,
            unique_refs: resp.unique_refs,
            custom_l0_count: resp.custom_l0_count,
            reused_concepts: resp.reused_concepts,
        })
    }
}

fn to_pb_episode(ep: MemoryEpisode) -> pb::Episode {
    pb::Episode {
        text: ep.text,
        concepts: ep
            .concepts
            .into_iter()
            .map(|c| pb::Concept {
                text: c.text,
                confidence: c.confidence,
                anchors: c
                    .anchors
                    .into_iter()
                    .map(|a| pb::Anchor { text: a.text, role: a.role, strength: a.strength })
                    .collect(),
            })
            .collect(),
    }
}
