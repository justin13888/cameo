//! Provider extension traits.
//!
//! Per ADR 0001 these are the v1 extension seam: object-safe and `Send` (via
//! `#[async_trait]`), returning the unified [`ProviderError`](crate::ProviderError),
//! taking [`MediaId`](crate::MediaId) parameters, and yielding the
//! provider-agnostic [`Page`](crate::Page). Concrete providers
//! (`TmdbClient`, `AniListClient`, and out-of-crate providers) implement them,
//! so `Arc<dyn SearchProvider>` and friends are constructible.

use async_trait::async_trait;

use super::{
    media_id::MediaId,
    models::{
        UnifiedEpisode, UnifiedMovie, UnifiedMovieDetails, UnifiedPerson, UnifiedPersonDetails,
        UnifiedSearchResult, UnifiedSeasonDetails, UnifiedTvShow, UnifiedTvShowDetails,
        UnifiedWatchProviders,
    },
};
use crate::core::{error::ProviderError, pagination::Page};

/// A capability a provider may support. Used by the facade to distinguish
/// "no provider configured" from "no configured provider supports this".
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Capability {
    /// Search for movies, TV shows, and people.
    Search,
    /// Fetch details for a single item.
    Details,
    /// Discover trending / popular / top-rated content.
    Discovery,
    /// Fetch recommendations and similar content.
    Recommendations,
    /// Fetch season and episode details.
    Seasons,
    /// Fetch streaming availability.
    WatchAvailability,
}

/// The base trait every provider implements: its identity and capabilities.
///
/// A supertrait of the capability traits, so `dyn SearchProvider` (etc.) can be
/// interrogated for its provider id and supported capabilities.
pub trait Provider: Send + Sync {
    /// The provider's short identifier (e.g. `"tmdb"`, `"anilist"`). Must match
    /// the [`MediaId::provider`](crate::unified::MediaId::provider) tag the
    /// provider mints, so the facade can route id-origin lookups.
    fn id(&self) -> &str;

    /// The capabilities this provider supports.
    fn capabilities(&self) -> &[Capability];

    /// Whether this provider supports `capability`.
    fn supports(&self, capability: Capability) -> bool {
        self.capabilities().contains(&capability)
    }
}

/// Provider that can search for movies, TV shows, and people.
#[async_trait]
pub trait SearchProvider: Provider {
    /// Search for movies by title.
    async fn search_movies(
        &self,
        query: &str,
        page: Option<u32>,
    ) -> Result<Page<UnifiedMovie>, ProviderError>;

    /// Search for TV shows by name.
    async fn search_tv_shows(
        &self,
        query: &str,
        page: Option<u32>,
    ) -> Result<Page<UnifiedTvShow>, ProviderError>;

    /// Search for people by name.
    async fn search_people(
        &self,
        query: &str,
        page: Option<u32>,
    ) -> Result<Page<UnifiedPerson>, ProviderError>;

    /// Multi-search across movies, TV shows, and people.
    async fn search_multi(
        &self,
        query: &str,
        page: Option<u32>,
    ) -> Result<Page<UnifiedSearchResult>, ProviderError>;
}

/// Provider that can fetch detailed information about individual items.
#[async_trait]
pub trait DetailProvider: Provider {
    /// Get detailed information about a movie by [`MediaId`].
    async fn movie_details(&self, id: &MediaId) -> Result<UnifiedMovieDetails, ProviderError>;

    /// Get detailed information about a TV show by [`MediaId`].
    async fn tv_show_details(&self, id: &MediaId) -> Result<UnifiedTvShowDetails, ProviderError>;

    /// Get detailed information about a person by [`MediaId`].
    async fn person_details(&self, id: &MediaId) -> Result<UnifiedPersonDetails, ProviderError>;
}

/// Provider that can discover trending and popular content.
#[async_trait]
pub trait DiscoveryProvider: Provider {
    /// Get trending movies.
    ///
    /// `time_window` (day/week) is a TMDB concept; providers without the notion
    /// (e.g. AniList) document that they ignore it.
    async fn trending_movies(
        &self,
        time_window: crate::core::config::TimeWindow,
        page: Option<u32>,
    ) -> Result<Page<UnifiedMovie>, ProviderError>;

    /// Get trending TV shows.
    async fn trending_tv_shows(
        &self,
        time_window: crate::core::config::TimeWindow,
        page: Option<u32>,
    ) -> Result<Page<UnifiedTvShow>, ProviderError>;

    /// Get popular movies.
    async fn popular_movies(&self, page: Option<u32>) -> Result<Page<UnifiedMovie>, ProviderError>;

    /// Get top-rated movies.
    async fn top_rated_movies(
        &self,
        page: Option<u32>,
    ) -> Result<Page<UnifiedMovie>, ProviderError>;

    /// Get popular TV shows.
    async fn popular_tv_shows(
        &self,
        page: Option<u32>,
    ) -> Result<Page<UnifiedTvShow>, ProviderError>;

    /// Get top-rated TV shows.
    async fn top_rated_tv_shows(
        &self,
        page: Option<u32>,
    ) -> Result<Page<UnifiedTvShow>, ProviderError>;
}

/// Provider that can fetch recommendations and similar content.
#[async_trait]
pub trait RecommendationProvider: Provider {
    /// Get movie recommendations based on a movie.
    async fn movie_recommendations(
        &self,
        id: &MediaId,
        page: Option<u32>,
    ) -> Result<Page<UnifiedMovie>, ProviderError>;

    /// Get TV show recommendations based on a TV show.
    async fn tv_recommendations(
        &self,
        id: &MediaId,
        page: Option<u32>,
    ) -> Result<Page<UnifiedTvShow>, ProviderError>;

    /// Get movies similar to a given movie.
    async fn similar_movies(
        &self,
        id: &MediaId,
        page: Option<u32>,
    ) -> Result<Page<UnifiedMovie>, ProviderError>;

    /// Get TV shows similar to a given TV show.
    async fn similar_tv_shows(
        &self,
        id: &MediaId,
        page: Option<u32>,
    ) -> Result<Page<UnifiedTvShow>, ProviderError>;
}

/// Provider that can fetch season and episode details.
#[async_trait]
pub trait SeasonProvider: Provider {
    /// Get details for a specific season of a TV show.
    async fn season_details(
        &self,
        show_id: &MediaId,
        season_number: u32,
    ) -> Result<UnifiedSeasonDetails, ProviderError>;

    /// Get details for a specific episode of a TV show.
    async fn episode_details(
        &self,
        show_id: &MediaId,
        season_number: u32,
        episode_number: u32,
    ) -> Result<UnifiedEpisode, ProviderError>;
}

/// Provider that can fetch streaming availability.
#[async_trait]
pub trait WatchAvailabilityProvider: Provider {
    /// Get streaming providers for a movie.
    async fn movie_watch_providers(
        &self,
        id: &MediaId,
    ) -> Result<UnifiedWatchProviders, ProviderError>;

    /// Get streaming providers for a TV show.
    async fn tv_watch_providers(
        &self,
        id: &MediaId,
    ) -> Result<UnifiedWatchProviders, ProviderError>;
}

/// A provider that supports search, detail, and discovery operations.
pub trait MediaProvider: SearchProvider + DetailProvider + DiscoveryProvider {}

/// Blanket implementation: anything that implements all three traits is a `MediaProvider`.
impl<T> MediaProvider for T where T: SearchProvider + DetailProvider + DiscoveryProvider {}
