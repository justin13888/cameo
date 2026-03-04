/// Conversions from AniList GraphQL response types to unified types.
#[cfg(feature = "anilist")]
pub mod anilist;

/// Conversions from TMDB generated types to unified types.
#[cfg(feature = "tmdb")]
pub mod tmdb;
