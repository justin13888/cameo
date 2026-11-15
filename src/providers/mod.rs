/// AniList anime database provider.
#[cfg(feature = "anilist")]
pub mod anilist;

/// Shared HTTP helpers (client construction, `Retry-After`, retryable statuses).
#[cfg(any(feature = "tmdb", feature = "anilist", feature = "tvdb"))]
pub(crate) mod http;

/// Per-provider request-rate limiting.
#[cfg(any(feature = "tmdb", feature = "anilist", feature = "tvdb"))]
pub(crate) mod rate_limit;

/// Retry policy for transient provider failures.
#[cfg(any(feature = "tmdb", feature = "anilist", feature = "tvdb"))]
pub(crate) mod retry;

#[cfg(any(feature = "tmdb", feature = "anilist", feature = "tvdb"))]
pub use rate_limit::RateLimit;
#[cfg(any(feature = "tmdb", feature = "anilist", feature = "tvdb"))]
pub use retry::RetryPolicy;

/// TMDB (The Movie Database) provider.
#[cfg(feature = "tmdb")]
pub mod tmdb;

/// TheTVDB (v4) provider.
#[cfg(feature = "tvdb")]
pub mod tvdb;
