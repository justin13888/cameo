use serde::Deserialize;

/// A GraphQL error object returned by the AniList API.
#[derive(Debug, Clone, Deserialize)]
pub struct AniListGqlError {
    /// Human-readable error message.
    pub message: String,
}

/// Error type for the AniList provider.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum AniListError {
    /// HTTP transport error.
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// One or more GraphQL errors returned by AniList.
    #[error("GraphQL errors: {}", .0.iter().map(|e| e.message.as_str()).collect::<Vec<_>>().join("; "))]
    GraphQL(Vec<AniListGqlError>),

    /// The API returned a successful response but no `data` field.
    #[error("no data returned from AniList")]
    NoData,

    /// The requested resource was not found.
    #[error("not found")]
    NotFound,
}
