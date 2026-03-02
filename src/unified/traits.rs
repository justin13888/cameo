use async_trait::async_trait;

use super::models::{
    UnifiedMovie, UnifiedMovieDetails, UnifiedPerson, UnifiedPersonDetails, UnifiedSearchResult,
    UnifiedTvShow, UnifiedTvShowDetails,
};
use crate::core::pagination::PaginatedResponse;

/// Provider that can search for movies, TV shows, and people.
#[async_trait]
pub trait SearchProvider {
    /// The error type returned by this provider.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Search for movies by title.
    async fn search_movies(
        &self,
        query: &str,
        page: Option<u32>,
    ) -> Result<PaginatedResponse<UnifiedMovie>, Self::Error>;

    /// Search for TV shows by name.
    async fn search_tv_shows(
        &self,
        query: &str,
        page: Option<u32>,
    ) -> Result<PaginatedResponse<UnifiedTvShow>, Self::Error>;

    /// Search for people by name.
    async fn search_people(
        &self,
        query: &str,
        page: Option<u32>,
    ) -> Result<PaginatedResponse<UnifiedPerson>, Self::Error>;

    /// Multi-search across movies, TV shows, and people.
    async fn search_multi(
        &self,
        query: &str,
        page: Option<u32>,
    ) -> Result<PaginatedResponse<UnifiedSearchResult>, Self::Error>;
}

/// Provider that can fetch detailed information about individual items.
#[async_trait]
pub trait DetailProvider {
    /// The error type returned by this provider.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Get detailed information about a movie by provider-specific ID.
    async fn movie_details(&self, id: i32) -> Result<UnifiedMovieDetails, Self::Error>;

    /// Get detailed information about a TV show by provider-specific ID.
    async fn tv_show_details(&self, id: i32) -> Result<UnifiedTvShowDetails, Self::Error>;

    /// Get detailed information about a person by provider-specific ID.
    async fn person_details(&self, id: i32) -> Result<UnifiedPersonDetails, Self::Error>;
}

/// Provider that can discover trending and popular content.
#[async_trait]
pub trait DiscoveryProvider {
    /// The error type returned by this provider.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Get trending movies.
    async fn trending_movies(
        &self,
        time_window: crate::core::config::TimeWindow,
        page: Option<u32>,
    ) -> Result<PaginatedResponse<UnifiedMovie>, Self::Error>;

    /// Get trending TV shows.
    async fn trending_tv_shows(
        &self,
        time_window: crate::core::config::TimeWindow,
        page: Option<u32>,
    ) -> Result<PaginatedResponse<UnifiedTvShow>, Self::Error>;

    /// Get popular movies.
    async fn popular_movies(
        &self,
        page: Option<u32>,
    ) -> Result<PaginatedResponse<UnifiedMovie>, Self::Error>;

    /// Get top-rated movies.
    async fn top_rated_movies(
        &self,
        page: Option<u32>,
    ) -> Result<PaginatedResponse<UnifiedMovie>, Self::Error>;
}

/// A provider that supports search, detail, and discovery operations.
pub trait MediaProvider: SearchProvider + DetailProvider + DiscoveryProvider {}

/// Blanket implementation: anything that implements all three traits is a `MediaProvider`.
impl<T> MediaProvider for T where T: SearchProvider + DetailProvider + DiscoveryProvider {}
