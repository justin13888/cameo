//! The leaf modules are private; every public item is re-exported here, which
//! is its canonical path (the most common ones are also at the crate root).

/// AniList GraphQL client.
pub(crate) mod client;
/// AniList client configuration.
pub(crate) mod config;
/// Conversions from AniList GraphQL response types to unified models, and the
/// format and page helpers the client builds requests and pages with.
mod convert;
/// Filtered anime discovery builder and its typed filter enums.
pub(crate) mod discover;
/// AniList-specific error types.
pub(crate) mod error;
/// Unified provider-trait implementations.
mod provider;
/// GraphQL query string constants.
pub(crate) mod query;
/// GraphQL response types (serde-deserializable).
pub(crate) mod response;

pub use client::AniListClient;
pub use config::AniListConfig;
pub use discover::{AnimeFormat, AnimeSeason, AnimeSort, AnimeStatus, DiscoverAnimeBuilder};
pub use error::{AniListError, AniListErrorLocation, AniListGqlError};
