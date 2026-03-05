use async_trait::async_trait;

use super::models::{
    UnifiedEpisode, UnifiedMovie, UnifiedMovieDetails, UnifiedPerson, UnifiedPersonDetails,
    UnifiedSearchResult, UnifiedSeasonDetails, UnifiedTvShow, UnifiedTvShowDetails,
    UnifiedWatchProviders,
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

    /// Get popular TV shows.
    async fn popular_tv_shows(
        &self,
        page: Option<u32>,
    ) -> Result<PaginatedResponse<UnifiedTvShow>, Self::Error>;

    /// Get top-rated TV shows.
    async fn top_rated_tv_shows(
        &self,
        page: Option<u32>,
    ) -> Result<PaginatedResponse<UnifiedTvShow>, Self::Error>;
}

/// Provider that can fetch recommendations and similar content.
#[async_trait]
pub trait RecommendationProvider {
    /// The error type returned by this provider.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Get movie recommendations based on a movie.
    async fn movie_recommendations(
        &self,
        id: i32,
        page: Option<u32>,
    ) -> Result<PaginatedResponse<UnifiedMovie>, Self::Error>;

    /// Get TV show recommendations based on a TV show.
    async fn tv_recommendations(
        &self,
        id: i32,
        page: Option<u32>,
    ) -> Result<PaginatedResponse<UnifiedTvShow>, Self::Error>;

    /// Get movies similar to a given movie.
    async fn similar_movies(
        &self,
        id: i32,
        page: Option<u32>,
    ) -> Result<PaginatedResponse<UnifiedMovie>, Self::Error>;

    /// Get TV shows similar to a given TV show.
    async fn similar_tv_shows(
        &self,
        id: i32,
        page: Option<u32>,
    ) -> Result<PaginatedResponse<UnifiedTvShow>, Self::Error>;
}

/// Provider that can fetch season and episode details.
#[async_trait]
pub trait SeasonProvider {
    /// The error type returned by this provider.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Get details for a specific season of a TV show.
    async fn season_details(
        &self,
        show_id: i32,
        season_number: u32,
    ) -> Result<UnifiedSeasonDetails, Self::Error>;

    /// Get details for a specific episode of a TV show.
    async fn episode_details(
        &self,
        show_id: i32,
        season_number: u32,
        episode_number: u32,
    ) -> Result<UnifiedEpisode, Self::Error>;
}

/// Provider that can fetch streaming availability.
#[async_trait]
pub trait WatchProviderTrait {
    /// The error type returned by this provider.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Get streaming providers for a movie.
    async fn movie_watch_providers(&self, id: i32) -> Result<UnifiedWatchProviders, Self::Error>;

    /// Get streaming providers for a TV show.
    async fn tv_watch_providers(&self, id: i32) -> Result<UnifiedWatchProviders, Self::Error>;
}

/// A provider that supports search, detail, and discovery operations.
pub trait MediaProvider: SearchProvider + DetailProvider + DiscoveryProvider {}

/// Blanket implementation: anything that implements all three traits is a `MediaProvider`.
impl<T> MediaProvider for T where T: SearchProvider + DetailProvider + DiscoveryProvider {}
