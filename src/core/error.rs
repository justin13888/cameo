/// Top-level error type for the cameo crate.
///
/// Wraps provider-specific errors so callers can handle them uniformly.
/// For the facade layer, see [`crate::unified::CameoClientError`], which
/// also covers configuration errors such as missing providers.
#[derive(Debug, thiserror::Error)]
pub enum CameoError {
    /// Error from the TMDB provider.
    #[cfg(feature = "tmdb")]
    #[error(transparent)]
    Tmdb(#[from] crate::providers::tmdb::error::TmdbError),

    /// Error from the AniList provider.
    #[cfg(feature = "anilist")]
    #[error(transparent)]
    AniList(#[from] crate::providers::anilist::AniListError),

    /// A provider returned an invalid or unexpected response.
    #[error("invalid response: {0}")]
    InvalidResponse(String),
}
