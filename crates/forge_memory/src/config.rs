//! Crate-owned memory configuration. Deliberately independent of core's provider
//! credential system so the memory layer adds zero coupling to forgecode core.
//!
//! Read from `<base_path>/memory.json`, where `<base_path>` is the same config
//! dir core uses (`~/.mnethos`, or `MNETHOS_CONFIG` when set) — so test
//! isolation via `MNETHOS_CONFIG` keeps working.

use serde::Deserialize;

use crate::client::MemoryClient;

const MEMORY_CONFIG_FILE: &str = "memory.json";

/// Connection details for the memory server.
#[derive(Debug, Clone, Deserialize)]
pub struct MemoryConfig {
    /// gRPC endpoint of the memory server (e.g. `http://localhost:8084`).
    pub server_url: String,
    /// Bearer token (awm_ API key) for the memory server.
    pub token: String,
}

impl MemoryConfig {
    /// Loads `<base_path>/memory.json`. Returns `Ok(None)` when the file is
    /// absent (memory then no-ops) so normal runs are unaffected. `Err` only on
    /// a present-but-malformed file.
    pub fn load() -> anyhow::Result<Option<Self>> {
        let path = forge_config::ConfigReader::base_path().join(MEMORY_CONFIG_FILE);
        let bytes = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(err) => return Err(anyhow::anyhow!("reading {}: {err}", path.display())),
        };
        let cfg: MemoryConfig = serde_json::from_slice(&bytes)
            .map_err(|err| anyhow::anyhow!("parsing {}: {err}", path.display()))?;
        Ok(Some(cfg))
    }

    /// A ready-to-use gRPC client for this config.
    pub fn client(&self) -> MemoryClient {
        MemoryClient::new(self.server_url.clone(), self.token.clone())
    }
}
