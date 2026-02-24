/// Top-level error type for the cameo crate.
#[derive(Debug, thiserror::Error)]
pub enum CameoError {
    /// Error from the TMDB provider.
    #[cfg(feature = "tmdb")]
    #[error(transparent)]
    Tmdb(#[from] crate::providers::tmdb::error::TmdbError),

    /// A provider returned an invalid or unexpected response.
    #[error("invalid response: {0}")]
    InvalidResponse(String),
}
