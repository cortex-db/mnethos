use anyhow::{Context, Result};
use forge_domain::{
    MemoryAnchor, MemoryConcept, MemoryEpisode, MemoryRecallItem, MemoryWriteResult,
};
use tonic::transport::{Channel, ClientTlsConfig};

use crate::memorywrite_proto::memory_write_client::MemoryWriteClient;
use crate::memorywrite_proto::{self as pb};

/// Connection details for the ai-working-memory backend, resolved by the caller
/// from the `mnethos_memory` provider credential (token) and provider URL
/// (`server_url`). Passed per call so this client stays stateless.
#[derive(Debug, Clone)]
pub struct MemoryConn {
    /// gRPC endpoint of the memory backend (e.g. `https://awm.mnethos.com:8084`).
    pub server_url: String,
    /// Bearer token (the `mnethos_memory` API key) sent as `authorization`.
    pub token: String,
}

impl MemoryConn {
    /// Creates a new connection descriptor.
    pub fn new(server_url: impl Into<String>, token: impl Into<String>) -> Self {
        Self { server_url: server_url.into(), token: token.into() }
    }
}

/// Stateless gRPC client for the ai-working-memory service (MemoryWrite), the
/// long-term memory backend. Mirrors [`crate::ForgeContextEngineRepository`]:
/// builds its own lazy channel and attaches `Bearer <token>` metadata. The
/// connection config ([`MemoryConn`]) is supplied per call by
/// [`crate::ForgeRepo`], which sources it from the `mnethos_memory` provider
/// credential; when that provider is unconfigured the repo never calls this
/// client, keeping memory a no-op.
#[derive(Default)]
pub struct ForgeMemoryRepository;

impl ForgeMemoryRepository {
    /// Creates a new stateless memory client.
    pub fn new() -> Self {
        Self
    }

    /// Persists one episode into the caller's session over gRPC.
    ///
    /// # Arguments
    /// * `conn` - The resolved memory backend connection (URL + token).
    /// * `session_key` - Stable per-project memory session key.
    /// * `episode` - The episode to persist.
    ///
    /// # Errors
    /// Returns an error if the channel cannot be built or the gRPC call fails.
    pub async fn create_episode(
        &self,
        conn: &MemoryConn,
        session_key: &str,
        episode: MemoryEpisode,
    ) -> Result<MemoryWriteResult> {
        let mut client = MemoryWriteClient::new(build_channel(&conn.server_url)?);
        let mut request = tonic::Request::new(pb::CreateEpisodeRequest {
            session_key: session_key.to_string(),
            episode: Some(to_pb_episode(episode)),
        });
        request
            .metadata_mut()
            .insert("authorization", bearer(&conn.token)?);

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

    /// Targeted spreading-activation retrieval over gRPC.
    ///
    /// # Arguments
    /// * `conn` - The resolved memory backend connection (URL + token).
    /// * `session_key` - Stable per-project memory session key.
    /// * `anchors` - Statement-shape English phrases to activate from.
    ///
    /// # Errors
    /// Returns an error if the channel cannot be built or the gRPC call fails.
    pub async fn retrieve(
        &self,
        conn: &MemoryConn,
        session_key: &str,
        anchors: Vec<String>,
    ) -> Result<Vec<MemoryRecallItem>> {
        let mut client = MemoryWriteClient::new(build_channel(&conn.server_url)?);
        let mut request = tonic::Request::new(pb::RetrieveRequest {
            session_key: session_key.to_string(),
            anchors,
        });
        request
            .metadata_mut()
            .insert("authorization", bearer(&conn.token)?);

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

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn test_memory_conn_new_stores_fields() {
        let fixture = MemoryConn::new("https://awm.mnethos.com:8084", "awm_token");
        let actual = (fixture.server_url, fixture.token);
        let expected = (
            "https://awm.mnethos.com:8084".to_string(),
            "awm_token".to_string(),
        );
        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn test_build_channel_accepts_http_url() {
        let actual = build_channel("http://localhost:8084").is_ok();
        assert!(actual);
    }

    #[test]
    fn test_build_channel_rejects_invalid_url() {
        let actual = build_channel("not a url").is_err();
        assert!(actual);
    }
}
