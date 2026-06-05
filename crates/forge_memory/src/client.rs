use anyhow::{Context, Result};
use tonic::transport::{Channel, ClientTlsConfig};

use crate::domain::{MemoryEpisode, MemoryRecallItem, MemoryWriteResult};
use crate::proto::memory_write_client::MemoryWriteClient;
use crate::proto::{self as pb};

/// gRPC client for the ai-working-memory memory service (MemoryWrite).
///
/// Self-contained: builds its own lazy channel to the memory server URL (from
/// `memory.json`), independent of any forgecode service channel. Auth:
/// `Bearer <token>` request metadata. Covers both write (`create_episode`) and
/// the fast read (`retrieve`, = MCP `retrieve_episodes`).
pub struct MemoryClient {
    server_url: String,
    token: String,
}

impl MemoryClient {
    pub fn new(server_url: impl Into<String>, token: impl Into<String>) -> Self {
        Self { server_url: server_url.into(), token: token.into() }
    }

    /// Builds a lazily-connected channel, enabling TLS for https URLs.
    fn channel(&self) -> Result<Channel> {
        let endpoint =
            Channel::from_shared(self.server_url.clone()).context("invalid memory server URL")?;
        let endpoint = if self.server_url.starts_with("https://") {
            endpoint
                .tls_config(ClientTlsConfig::new().with_webpki_roots())
                .context("failed to configure TLS for memory channel")?
        } else {
            endpoint
        };
        Ok(endpoint.connect_lazy())
    }

    fn bearer(&self) -> Result<tonic::metadata::MetadataValue<tonic::metadata::Ascii>> {
        Ok(format!("Bearer {}", self.token).parse()?)
    }

    /// Create one episode, scoped by `session_key` (server does get-or-create of
    /// a session per (owner, session_key)).
    pub async fn create_episode(
        &self,
        session_key: &str,
        episode: MemoryEpisode,
    ) -> Result<MemoryWriteResult> {
        let mut client = MemoryWriteClient::new(self.channel()?);
        let mut request = tonic::Request::new(pb::CreateEpisodeRequest {
            session_key: session_key.to_string(),
            episode: Some(to_pb_episode(episode)),
        });
        request.metadata_mut().insert("authorization", self.bearer()?);

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

    /// Targeted spreading-activation retrieval (the fast route, = MCP
    /// `retrieve_episodes`). `anchors` are statement-shape English phrases; a
    /// missing session yields an empty Vec.
    pub async fn retrieve(
        &self,
        session_key: &str,
        anchors: Vec<String>,
    ) -> Result<Vec<MemoryRecallItem>> {
        let mut client = MemoryWriteClient::new(self.channel()?);
        let mut request = tonic::Request::new(pb::RetrieveRequest {
            session_key: session_key.to_string(),
            anchors,
        });
        request.metadata_mut().insert("authorization", self.bearer()?);

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
