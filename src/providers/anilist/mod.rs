/// AniList GraphQL client.
pub mod client;
/// AniList client configuration.
pub mod config;
/// Filtered anime discovery builder and its typed filter enums.
pub mod discover;
/// AniList-specific error types.
pub mod error;
/// Unified provider-trait implementations.
mod provider;
/// GraphQL query string constants.
pub(crate) mod query;
/// GraphQL response types (serde-deserializable).
pub(crate) mod response;

pub use client::AniListClient;
pub use config::AniListConfig;
pub use discover::{AnimeFormat, AnimeSeason, AnimeSort, AnimeStatus, DiscoverAnimeBuilder};
pub use error::AniListError;
