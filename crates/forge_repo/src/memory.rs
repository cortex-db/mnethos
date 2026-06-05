use anyhow::{Context, Result};
use async_trait::async_trait;
use forge_domain::{
    MemoryAnchor, MemoryConcept, MemoryEpisode, MemoryRecallItem, MemoryRepository,
    MemoryWriteResult,
};
use serde::Deserialize;
use tonic::transport::{Channel, ClientTlsConfig};

use crate::memorywrite_proto::memory_write_client::MemoryWriteClient;
use crate::memorywrite_proto::{self as pb};

const MEMORY_CONFIG_FILE: &str = "memory.json";

/// gRPC client for the ai-working-memory service (MemoryWrite), the long-term
/// memory backend. Mirrors [`crate::ForgeContextEngineRepository`]: self-contained,
/// builds its own lazy channel, `Bearer <token>` metadata. Connection config is
/// read from `<base_path>/memory.json`; absent config makes every call a no-op so
/// memory stays inert when unconfigured.
#[derive(Default)]
pub struct ForgeMemoryRepository;

impl ForgeMemoryRepository {
    pub fn new() -> Self {
        Self
    }
}

/// `<base_path>/memory.json` — the memory backend connection details.
#[derive(Debug, Clone, Deserialize)]
struct MemoryConfig {
    server_url: String,
    token: String,
}

/// Loads `memory.json`, or `None` when absent (→ memory is a no-op).
fn load_config() -> Result<Option<MemoryConfig>> {
    let path = forge_config::ConfigReader::base_path().join(MEMORY_CONFIG_FILE);
    match std::fs::read(&path) {
        Ok(bytes) => {
            let cfg = serde_json::from_slice(&bytes)
                .with_context(|| format!("parsing {}", path.display()))?;
            Ok(Some(cfg))
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(anyhow::anyhow!("reading {}: {err}", path.display())),
    }
}

fn build_channel(server_url: &str) -> Result<Channel> {
    let endpoint =
        Channel::from_shared(server_url.to_string()).context("invalid memory server URL")?;
    let endpoint = if server_url.starts_with("https://") {
        endpoint
            .tls_config(ClientTlsConfig::new().with_webpki_roots())
            .context("failed to configure TLS for memory channel")?
    } else {
        endpoint
    };
    Ok(endpoint.connect_lazy())
}

fn bearer(token: &str) -> Result<tonic::metadata::MetadataValue<tonic::metadata::Ascii>> {
    Ok(format!("Bearer {token}").parse()?)
}

#[async_trait]
impl MemoryRepository for ForgeMemoryRepository {
    async fn create_episode(
        &self,
        session_key: &str,
        episode: MemoryEpisode,
    ) -> Result<Option<MemoryWriteResult>> {
        let Some(cfg) = load_config()? else {
            return Ok(None);
        };
        let mut client = MemoryWriteClient::new(build_channel(&cfg.server_url)?);
        let mut request = tonic::Request::new(pb::CreateEpisodeRequest {
            session_key: session_key.to_string(),
            episode: Some(to_pb_episode(episode)),
        });
        request.metadata_mut().insert("authorization", bearer(&cfg.token)?);

        let resp = client
            .create_episode(request)
            .await
            .context("memory CreateEpisode gRPC call failed")?
            .into_inner();

        Ok(Some(MemoryWriteResult {
            episode_id: resp.episode_id,
            concept_count: resp.concept_count,
            anchor_count: resp.anchor_count,
            unique_refs: resp.unique_refs,
            custom_l0_count: resp.custom_l0_count,
            reused_concepts: resp.reused_concepts,
        }))
    }

    async fn retrieve(
        &self,
        session_key: &str,
        anchors: Vec<String>,
    ) -> Result<Vec<MemoryRecallItem>> {
        let Some(cfg) = load_config()? else {
            return Ok(Vec::new());
        };
        let mut client = MemoryWriteClient::new(build_channel(&cfg.server_url)?);
        let mut request = tonic::Request::new(pb::RetrieveRequest {
            session_key: session_key.to_string(),
            anchors,
        });
        request.metadata_mut().insert("authorization", bearer(&cfg.token)?);

        let resp = client
            .retrieve(request)
            .await
            .context("memory Retrieve gRPC call failed")?
            .into_inner();

        Ok(resp
            .items
            .into_iter()
            .map(|it| MemoryRecallItem {
                node_id: it.node_id,
                level: it.level,
                activation: it.activation,
                created_at: it.created_at,
                aliases: it.aliases,
                confidence: it.confidence,
            })
            .collect())
    }
}

fn to_pb_episode(ep: MemoryEpisode) -> pb::Episode {
    pb::Episode {
        text: ep.text,
        concepts: ep.concepts.into_iter().map(to_pb_concept).collect(),
    }
}

fn to_pb_concept(c: MemoryConcept) -> pb::Concept {
    pb::Concept {
        text: c.text,
        confidence: c.confidence,
        anchors: c.anchors.into_iter().map(to_pb_anchor).collect(),
    }
}

fn to_pb_anchor(a: MemoryAnchor) -> pb::Anchor {
    pb::Anchor { text: a.text, role: a.role, strength: a.strength }
}
