//! Error types for the Mnethos reference server.

/// Errors that can occur while running the HTTP server.
#[derive(Debug, thiserror::Error)]
pub enum ServerError {
    /// The listening socket could not be bound to the requested address.
    #[error("failed to bind server to address `{address}`: {message}")]
    Bind {
        /// The address the server attempted to bind to.
        address: String,
        /// Human-readable description of the underlying failure.
        message: String,
    },

    /// An I/O error occurred while accepting or responding to a request.
    #[error("server I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Embedding generation via the ai-gateway failed.
    #[error("embedding error: {message}")]
    Embedding {
        /// Human-readable description of the failure.
        message: String,
    },

    /// A persistence operation against the workspace store failed.
    #[error("store error: {message}")]
    Store {
        /// Human-readable description of the failure.
        message: String,
    },
}
