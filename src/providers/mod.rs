/// AniList anime/manga database provider.
#[cfg(feature = "anilist")]
pub mod anilist;

/// Shared HTTP client construction (timeouts, `User-Agent`).
#[cfg(any(feature = "tmdb", feature = "anilist"))]
pub(crate) mod http;

/// TMDB (The Movie Database) provider.
#[cfg(feature = "tmdb")]
pub mod tmdb;
