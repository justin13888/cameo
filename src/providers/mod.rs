/// AniList anime/manga database provider.
#[cfg(feature = "anilist")]
pub mod anilist;

/// Shared HTTP client construction (timeouts, `User-Agent`).
#[cfg(any(feature = "tmdb", feature = "anilist"))]
pub(crate) mod http;

/// Per-provider request-rate limiting.
#[cfg(any(feature = "tmdb", feature = "anilist"))]
pub mod rate_limit;

/// TMDB (The Movie Database) provider.
#[cfg(feature = "tmdb")]
pub mod tmdb;
