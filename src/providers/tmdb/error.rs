/// Error type for the TMDB provider.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum TmdbError {
    /// Error from the underlying HTTP client.
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// Error from the progenitor-generated client, with the HTTP status code.
    ///
    /// The `status` field carries the HTTP response code (e.g. 401, 404) so
    /// callers can distinguish authentication failures from missing resources
    /// without parsing the message string.
    #[error("API error {status}: {message}")]
    Api {
        /// HTTP response status code.
        status: u16,
        /// Human-readable error description from the API or client library.
        message: String,
    },

    /// JSON deserialization error.
    #[error("deserialization error: {0}")]
    Deserialization(#[from] serde_json::Error),

    /// Exceeded the configured per-client request concurrency timeout.
    ///
    /// Raised when `TmdbConfig::rate_limit_timeout` is set and a semaphore
    /// permit could not be acquired within that duration, indicating that all
    /// concurrent request slots are occupied.
    #[error("rate limit exceeded: no permit available within timeout")]
    RateLimitExceeded,

    /// The rate-limit semaphore has been closed (internal error).
    ///
    /// This should not occur in normal operation; it indicates that the
    /// internal concurrency semaphore was unexpectedly dropped.
    #[error("internal error: rate-limit semaphore closed")]
    Closed,

    /// Invalid configuration.
    #[error("invalid configuration: {0}")]
    InvalidConfig(String),
}

impl<T: std::fmt::Debug + Send + Sync + 'static> From<progenitor_client::Error<T>> for TmdbError {
    fn from(err: progenitor_client::Error<T>) -> Self {
        // `progenitor_client::Error::status()` returns `Option<reqwest::StatusCode>`,
        // covering both `ErrorResponse` and `UnexpectedResponse` variants.
        let status = err.status().map(|s| s.as_u16()).unwrap_or(0);
        TmdbError::Api {
            status,
            message: format!("{err}"),
        }
    }
}
