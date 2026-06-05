//! Retry for the memory layer's one-shot model calls (anchor extraction +
//! episode consolidation). The core agent loop already retries transient API
//! failures; these out-of-loop calls did not, so a single flaky-API blip
//! dropped recall/write. Mirrors core's policy: retry ONLY on transient
//! (`Error::Retryable`) errors, with the same exponential backoff config.

use std::time::Duration;

use backon::{ExponentialBuilder, Retryable};
use forge_config::RetryConfig;
use forge_domain::Error;

/// Retries `op` with exponential backoff on transient errors only.
pub(crate) async fn with_retry<F, Fut, T>(config: &RetryConfig, op: F) -> anyhow::Result<T>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = anyhow::Result<T>>,
{
    let strategy = ExponentialBuilder::default()
        .with_min_delay(Duration::from_millis(config.min_delay_ms))
        .with_factor(config.backoff_factor as f32)
        .with_max_times(config.max_attempts)
        .with_jitter();
    op.retry(&strategy).when(is_retryable).await
}

/// Same classification as core's agent loop: only `Error::Retryable` (transient
/// provider/network failures) triggers a retry; permanent errors fail fast.
fn is_retryable(error: &anyhow::Error) -> bool {
    error
        .downcast_ref::<Error>()
        .is_some_and(|error| matches!(error, Error::Retryable(_)))
}
