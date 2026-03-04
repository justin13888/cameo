/// AniList GraphQL client.
pub mod client;
/// AniList client configuration.
pub mod config;
/// AniList-specific error types.
pub mod error;
/// GraphQL query string constants.
pub mod query;
/// GraphQL response types (serde-deserializable).
pub mod response;

pub use client::AniListClient;
pub use config::AniListConfig;
pub use error::AniListError;
