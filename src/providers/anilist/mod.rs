/// AniList GraphQL client.
pub mod client;
/// AniList-specific error types.
pub mod error;
/// GraphQL query string constants.
pub mod query;
/// GraphQL response types (serde-deserializable).
pub mod response;

pub use client::{AniListClient, AniListConfig};
pub use error::AniListError;
