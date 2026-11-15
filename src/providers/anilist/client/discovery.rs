//! Ranked lists (trending, popular, top-rated), recommendations, and
//! filtered discovery.

use serde_json::{Value, json};

use super::AniListClient;
use crate::{
    core::{config::TimeWindow, pagination::Page},
    providers::anilist::{
        convert::{
            MOVIE_FORMATS, TV_FORMATS, anilist_media_to_movie, anilist_media_to_tv, media_page,
            media_page_root, recommendations_page,
        },
        discover::DiscoverAnimeBuilder,
        error::AniListError,
        query,
        response::{MediaRecommendationsResponse, RecommendationConnection},
    },
    unified::models::{UnifiedMovie, UnifiedSearchResult, UnifiedTvShow},
};

impl AniListClient {
    /// Get trending anime movies.
    ///
    /// # Note
    ///
    /// AniList has no time-window concept. The `time_window` argument is
    /// accepted for trait compatibility but is always ignored — AniList
    /// returns a single global trending list regardless of the requested window.
    pub async fn trending_movies(
        &self,
        _time_window: TimeWindow,
        page: Option<u32>,
    ) -> Result<Page<UnifiedMovie>, AniListError> {
        let (page_num, vars) = self.list_vars(page, MOVIE_FORMATS);
        tracing::debug!(
            page = page_num,
            "anilist: graphql LIST_TRENDING_ANIME (trending_movies)"
        );
        let page = self
            .graphql(query::LIST_TRENDING_ANIME, vars, media_page_root)
            .await?;
        media_page(page, anilist_media_to_movie)
    }

    /// Get trending anime series.
    ///
    /// # Note
    ///
    /// AniList has no time-window concept. The `time_window` argument is
    /// accepted for trait compatibility but is always ignored — AniList
    /// returns a single global trending list regardless of the requested window.
    pub async fn trending_tv(
        &self,
        _time_window: TimeWindow,
        page: Option<u32>,
    ) -> Result<Page<UnifiedTvShow>, AniListError> {
        let (page_num, vars) = self.list_vars(page, TV_FORMATS);
        tracing::debug!(
            page = page_num,
            "anilist: graphql LIST_TRENDING_ANIME (trending_tv)"
        );
        let page = self
            .graphql(query::LIST_TRENDING_ANIME, vars, media_page_root)
            .await?;
        media_page(page, anilist_media_to_tv)
    }

    /// Get popular anime movies (sorted by popularity).
    pub async fn popular_movies(
        &self,
        page: Option<u32>,
    ) -> Result<Page<UnifiedMovie>, AniListError> {
        let (page_num, vars) = self.list_vars(page, MOVIE_FORMATS);
        tracing::debug!(
            page = page_num,
            "anilist: graphql LIST_POPULAR_ANIME (popular_movies)"
        );
        let page = self
            .graphql(query::LIST_POPULAR_ANIME, vars, media_page_root)
            .await?;
        media_page(page, anilist_media_to_movie)
    }

    /// Get top-scored anime movies (sorted by average score).
    pub async fn top_rated_movies(
        &self,
        page: Option<u32>,
    ) -> Result<Page<UnifiedMovie>, AniListError> {
        let (page_num, vars) = self.list_vars(page, MOVIE_FORMATS);
        tracing::debug!(
            page = page_num,
            "anilist: graphql LIST_TOP_SCORED_ANIME (top_rated_movies)"
        );
        let page = self
            .graphql(query::LIST_TOP_SCORED_ANIME, vars, media_page_root)
            .await?;
        media_page(page, anilist_media_to_movie)
    }

    /// Get popular anime TV shows (sorted by popularity).
    pub async fn popular_tv_shows(
        &self,
        page: Option<u32>,
    ) -> Result<Page<UnifiedTvShow>, AniListError> {
        let (page_num, vars) = self.list_vars(page, TV_FORMATS);
        tracing::debug!(
            page = page_num,
            "anilist: graphql LIST_POPULAR_ANIME (popular_tv_shows)"
        );
        let page = self
            .graphql(query::LIST_POPULAR_ANIME, vars, media_page_root)
            .await?;
        media_page(page, anilist_media_to_tv)
    }

    /// Get top-scored anime TV shows (sorted by average score).
    pub async fn top_rated_tv_shows(
        &self,
        page: Option<u32>,
    ) -> Result<Page<UnifiedTvShow>, AniListError> {
        let (page_num, vars) = self.list_vars(page, TV_FORMATS);
        tracing::debug!(
            page = page_num,
            "anilist: graphql LIST_TOP_SCORED_ANIME (top_rated_tv_shows)"
        );
        let page = self
            .graphql(query::LIST_TOP_SCORED_ANIME, vars, media_page_root)
            .await?;
        media_page(page, anilist_media_to_tv)
    }

    /// Get community recommendations for an anime, keeping only recommended
    /// **movies** (format `MOVIE`).
    ///
    /// Recommendations are ordered by community rating (`RATING_DESC`).
    /// AniList cannot filter recommendations by format server-side, so
    /// non-movie entries (and entries with no format, or deleted/hidden
    /// recommended media) are dropped locally: a page may hold **fewer** items
    /// than the configured page size, or none, while `total_results`,
    /// `total_pages`, and `has_next_page` keep describing AniList's unfiltered
    /// recommendation list.
    ///
    /// # Errors
    ///
    /// Returns [`AniListError::NotFound`] if no anime has this id.
    pub async fn movie_recommendations(
        &self,
        id: i32,
        page: Option<u32>,
    ) -> Result<Page<UnifiedMovie>, AniListError> {
        let recs = self.recommendations(id, page).await?;
        Ok(recommendations_page(recs)?.filter_map(|item| match item {
            UnifiedSearchResult::Movie(movie) => Some(movie),
            _ => None,
        }))
    }

    /// Get community recommendations for an anime, keeping only recommended
    /// **series** (the TV formats `TV`, `TV_SHORT`, `ONA`, `OVA`, `SPECIAL`).
    ///
    /// Uses the same classification as [`search_multi`](Self::search_multi);
    /// see [`movie_recommendations`](Self::movie_recommendations) for how
    /// local filtering affects page sizes and pagination metadata.
    ///
    /// # Errors
    ///
    /// Returns [`AniListError::NotFound`] if no anime has this id.
    pub async fn tv_recommendations(
        &self,
        id: i32,
        page: Option<u32>,
    ) -> Result<Page<UnifiedTvShow>, AniListError> {
        let recs = self.recommendations(id, page).await?;
        Ok(recommendations_page(recs)?.filter_map(|item| match item {
            UnifiedSearchResult::TvShow(show) => Some(show),
            _ => None,
        }))
    }

    /// Fetch one page of raw recommendations for an anime.
    async fn recommendations(
        &self,
        id: i32,
        page: Option<u32>,
    ) -> Result<Option<RecommendationConnection>, AniListError> {
        let (page_num, per_page) = self.page_vars(page);
        let vars = json!({
            "id": id,
            "page": page_num,
            "perPage": per_page,
        });
        tracing::debug!(
            id,
            page = page_num,
            "anilist: graphql MEDIA_RECOMMENDATIONS"
        );
        let media = self
            .graphql(
                query::MEDIA_RECOMMENDATIONS,
                vars,
                |r: MediaRecommendationsResponse| r.media,
            )
            .await?
            .ok_or(AniListError::NotFound)?;
        Ok(media.recommendations)
    }

    /// Create a builder for discovering anime with filters (genres, country,
    /// broadcast season, format, status, adult flag, sort order).
    ///
    /// See [`DiscoverAnimeBuilder`] for the available filters and how the
    /// movie/TV terminal methods choose formats.
    pub fn discover_anime(&self) -> DiscoverAnimeBuilder<'_> {
        DiscoverAnimeBuilder::new(self)
    }

    /// Run [`query::DISCOVER_ANIME`] and convert the page to movies.
    pub(crate) async fn discover_movies_page(
        &self,
        vars: Value,
    ) -> Result<Page<UnifiedMovie>, AniListError> {
        tracing::debug!(
            page = %vars["page"],
            "anilist: graphql DISCOVER_ANIME (discover_anime movies)"
        );
        let page = self
            .graphql(query::DISCOVER_ANIME, vars, media_page_root)
            .await?;
        media_page(page, anilist_media_to_movie)
    }

    /// Run [`query::DISCOVER_ANIME`] and convert the page to TV shows.
    pub(crate) async fn discover_tv_page(
        &self,
        vars: Value,
    ) -> Result<Page<UnifiedTvShow>, AniListError> {
        tracing::debug!(
            page = %vars["page"],
            "anilist: graphql DISCOVER_ANIME (discover_anime tv)"
        );
        let page = self
            .graphql(query::DISCOVER_ANIME, vars, media_page_root)
            .await?;
        media_page(page, anilist_media_to_tv)
    }
}
