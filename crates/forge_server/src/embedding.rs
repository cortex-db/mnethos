//! Embedding generation via the ai-gateway gRPC service.
//!
//! The context engine never talks to OpenAI directly: it asks the ai-gateway
//! (`AIGateway.Embed`) to embed text with `text-embedding-3-large` (3072
//! dimensions). Centralizing embeddings in the gateway gives us idempotency,
//! batching, cost accounting and a single egress point for the OpenAI key.

use async_trait::async_trait;
use tonic::transport::Channel;

use crate::error::ServerError;
use crate::proto::ai_gateway::ai_gateway_client::AiGatewayClient;
use crate::proto::ai_gateway::EmbedRequest;

/// The embedding model served by the gateway. 3072-dimensional vectors.
pub const EMBEDDING_MODEL: &str = "text-embedding-3-large";

/// Dimensionality of [`EMBEDDING_MODEL`] vectors.
pub const EMBEDDING_DIMENSION: usize = 3072;

/// Produces embedding vectors for input text.
#[async_trait]
pub trait Embedder: Send + Sync {
    /// Embeds a batch of input strings, returning one vector per input in order.
    ///
    /// # Errors
    /// Returns [`ServerError::Embedding`] when the backend call fails or returns
    /// a vector count that does not match the number of inputs.
    async fn embed(&self, inputs: &[String]) -> Result<Vec<Vec<f32>>, ServerError>;
}

/// [`Embedder`] backed by the ai-gateway gRPC service.
#[derive(Clone)]
pub struct AiGatewayEmbedder {
    endpoint: String,
    model: String,
}

impl AiGatewayEmbedder {
    /// Creates an embedder targeting the gateway at `endpoint`
    /// (e.g. `http://ai-gateway:50054`), using [`EMBEDDING_MODEL`].
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self { endpoint: endpoint.into(), model: EMBEDDING_MODEL.to_string() }
    }

    /// Overrides the embedding model name sent to the gateway.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Opens a fresh gRPC channel to the gateway.
    async fn connect(&self) -> Result<AiGatewayClient<Channel>, ServerError> {
        AiGatewayClient::connect(self.endpoint.clone())
            .await
            .map_err(|error| ServerError::Embedding {
                message: format!("failed to connect to ai-gateway at {}: {error}", self.endpoint),
            })
    }
}

#[async_trait]
impl Embedder for AiGatewayEmbedder {
    async fn embed(&self, inputs: &[String]) -> Result<Vec<Vec<f32>>, ServerError> {
        if inputs.is_empty() {
            return Ok(Vec::new());
        }

        let mut client = self.connect().await?;

        let request = tonic::Request::new(EmbedRequest {
            external_id: uuid::Uuid::new_v4().to_string(),
            model: self.model.clone(),
            inputs: inputs.to_vec(),
        });

        let response = client.embed(request).await.map_err(|status| ServerError::Embedding {
            message: format!("ai-gateway Embed failed: {status}"),
        })?;

        let vectors: Vec<Vec<f32>> =
            response.into_inner().vectors.into_iter().map(|v| v.values).collect();

        if vectors.len() != inputs.len() {
            return Err(ServerError::Embedding {
                message: format!(
                    "ai-gateway returned {} vectors for {} inputs",
                    vectors.len(),
                    inputs.len()
                ),
            });
        }

        Ok(vectors)
    }
}

#[cfg(test)]
pub mod tests {
    use std::collections::HashMap;
    use std::sync::Mutex;

    use super::*;

    /// Deterministic in-memory [`Embedder`] for tests.
    ///
    /// Produces a low-dimensional, content-derived vector so cosine similarity
    /// is meaningful without contacting a real gateway.
    pub struct FakeEmbedder {
        cache: Mutex<HashMap<String, Vec<f32>>>,
        dimension: usize,
    }

    impl FakeEmbedder {
        pub fn new(dimension: usize) -> Self {
            Self { cache: Mutex::new(HashMap::new()), dimension }
        }

        /// Hashes text into a stable unit-length vector.
        fn vector_for(&self, text: &str) -> Vec<f32> {
            let mut values = vec![0f32; self.dimension];
            for (index, byte) in text.bytes().enumerate() {
                values[index % self.dimension] += byte as f32;
            }
            let norm = values.iter().map(|v| v * v).sum::<f32>().sqrt().max(1.0);
            values.iter_mut().for_each(|v| *v /= norm);
            values
        }
    }

    #[async_trait]
    impl Embedder for FakeEmbedder {
        async fn embed(&self, inputs: &[String]) -> Result<Vec<Vec<f32>>, ServerError> {
            let mut cache = self.cache.lock().unwrap();
            Ok(inputs
                .iter()
                .map(|input| {
                    cache.entry(input.clone()).or_insert_with(|| self.vector_for(input)).clone()
                })
                .collect())
        }
    }

    #[tokio::test]
    async fn test_fake_embedder_is_deterministic() {
        let fixture = FakeEmbedder::new(8);
        let actual = fixture.embed(&["hello".to_string()]).await.unwrap();
        let expected = fixture.embed(&["hello".to_string()]).await.unwrap();
        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn test_fake_embedder_one_vector_per_input() {
        let fixture = FakeEmbedder::new(8);
        let actual = fixture
            .embed(&["a".to_string(), "b".to_string(), "c".to_string()])
            .await
            .unwrap();
        assert_eq!(actual.len(), 3);
    }
}
