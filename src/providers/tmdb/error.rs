/// Error type for the TMDB provider.
#[derive(Debug, thiserror::Error)]
pub enum TmdbError {
    /// Error from the underlying HTTP client.
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// Error from the progenitor-generated client.
    #[error("API error: {0}")]
    Api(String),

    /// JSON deserialization error.
    #[error("deserialization error: {0}")]
    Deserialization(#[from] serde_json::Error),

    /// Rate limit exceeded.
    #[error("rate limit exceeded")]
    RateLimitExceeded,

    /// Invalid configuration.
    #[error("invalid configuration: {0}")]
    InvalidConfig(String),
}

impl<T: std::fmt::Debug + Send + Sync + 'static> From<progenitor_client::Error<T>> for TmdbError {
    fn from(err: progenitor_client::Error<T>) -> Self {
        TmdbError::Api(format!("{err}"))
    }
}
