use chrono::NaiveDate;

use super::params::{self, MovieReleaseType};
use crate::{
    core::pagination::Page,
    providers::tmdb::{TmdbClient, builders::MovieSortBy, error::TmdbError},
    unified::models::UnifiedMovie,
};

/// Builder for discovering movies with flexible filters.
///
/// The ~15 filters shared with [`DiscoverTvBuilder`](super::DiscoverTvBuilder)
/// come from the shared setter core in [`super::params`]; the
/// setters below are movie-specific. Provider ID filters take `&[u64]` slices
/// and coded filters take typed enums.
///
/// # Example
///
/// ```no_run
/// # async fn example(client: &cameo::providers::tmdb::TmdbClient) -> Result<(), cameo::providers::tmdb::TmdbError> {
/// use cameo::providers::tmdb::builders::MovieSortBy;
/// let results = client
///     .discover_movies()
///     .sort_by(MovieSortBy::PopularityDesc)
///     .with_genres(&[28]) // Action
///     .primary_release_year(2024)
///     .execute()
///     .await?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
#[must_use = "a discover query does nothing until .execute() is awaited"]
pub struct DiscoverMoviesBuilder<'a> {
    client: &'a TmdbClient,
    certification: Option<String>,
    certification_gte: Option<String>,
    certification_lte: Option<String>,
    certification_country: Option<String>,
    include_adult: Option<bool>,
    include_video: Option<bool>,
    language: Option<String>,
    page: Option<i32>,
    primary_release_date_gte: Option<NaiveDate>,
    primary_release_date_lte: Option<NaiveDate>,
    primary_release_year: Option<i32>,
    region: Option<String>,
    release_date_gte: Option<NaiveDate>,
    release_date_lte: Option<NaiveDate>,
    sort_by: Option<MovieSortBy>,
    vote_average_gte: Option<f32>,
    vote_average_lte: Option<f32>,
    vote_count_gte: Option<f32>,
    vote_count_lte: Option<f32>,
    watch_region: Option<String>,
    with_cast: Option<String>,
    with_companies: Option<String>,
    with_crew: Option<String>,
    with_genres: Option<String>,
    with_keywords: Option<String>,
    with_origin_country: Option<String>,
    with_original_language: Option<String>,
    with_people: Option<String>,
    with_release_type: Option<i32>,
    with_runtime_gte: Option<i32>,
    with_runtime_lte: Option<i32>,
    with_watch_monetization_types: Option<String>,
    with_watch_providers: Option<String>,
    without_companies: Option<String>,
    without_genres: Option<String>,
    without_keywords: Option<String>,
    without_watch_providers: Option<String>,
    year: Option<i32>,
}

impl<'a> DiscoverMoviesBuilder<'a> {
    pub(crate) fn new(client: &'a TmdbClient) -> Self {
        Self {
            client,
            certification: None,
            certification_gte: None,
            certification_lte: None,
            certification_country: None,
            include_adult: client.config().include_adult,
            include_video: None,
            language: client.config().language.clone(),
            page: None,
            primary_release_date_gte: None,
            primary_release_date_lte: None,
            primary_release_year: None,
            region: client.config().region.clone(),
            release_date_gte: None,
            release_date_lte: None,
            sort_by: None,
            vote_average_gte: None,
            vote_average_lte: None,
            vote_count_gte: None,
            vote_count_lte: None,
            watch_region: None,
            with_cast: None,
            with_companies: None,
            with_crew: None,
            with_genres: None,
            with_keywords: None,
            with_origin_country: None,
            with_original_language: None,
            with_people: None,
            with_release_type: None,
            with_runtime_gte: None,
            with_runtime_lte: None,
            with_watch_monetization_types: None,
            with_watch_providers: None,
            without_companies: None,
            without_genres: None,
            without_keywords: None,
            without_watch_providers: None,
            year: None,
        }
    }

    // The ~15 filters common to both discover builders, defined once.
    params::common_discover_setters!();

    /// Set the sort order.
    pub fn sort_by(mut self, sort_by: MovieSortBy) -> Self {
        self.sort_by = Some(sort_by);
        self
    }

    /// Filter by primary release year.
    pub fn primary_release_year(mut self, year: i32) -> Self {
        self.primary_release_year = Some(year);
        self
    }

    /// Filter by release year.
    pub fn year(mut self, year: i32) -> Self {
        self.year = Some(year);
        self
    }

    /// Primary release date on or after.
    pub fn primary_release_date_gte(mut self, date: NaiveDate) -> Self {
        self.primary_release_date_gte = Some(date);
        self
    }

    /// Primary release date on or before.
    pub fn primary_release_date_lte(mut self, date: NaiveDate) -> Self {
        self.primary_release_date_lte = Some(date);
        self
    }

    /// Filter by cast member IDs.
    pub fn with_cast(mut self, person_ids: &[u64]) -> Self {
        self.with_cast = Some(params::join_ids(person_ids));
        self
    }

    /// Filter by crew member IDs.
    pub fn with_crew(mut self, person_ids: &[u64]) -> Self {
        self.with_crew = Some(params::join_ids(person_ids));
        self
    }

    /// Filter by person IDs (cast or crew).
    pub fn with_people(mut self, person_ids: &[u64]) -> Self {
        self.with_people = Some(params::join_ids(person_ids));
        self
    }

    /// Filter by origin country (ISO 3166-1 alpha-2, e.g. `"US"`).
    pub fn with_origin_country(mut self, country: impl Into<String>) -> Self {
        self.with_origin_country = Some(country.into());
        self
    }

    /// Filter by release type.
    pub fn with_release_type(mut self, release_type: MovieReleaseType) -> Self {
        self.with_release_type = Some(release_type.code());
        self
    }

    /// Certification country (ISO 3166-1 alpha-2). Required when filtering by
    /// certification.
    pub fn certification_country(mut self, country: impl Into<String>) -> Self {
        self.certification_country = Some(country.into());
        self
    }

    /// Filter by exact certification rating (e.g. `"PG-13"`). Requires
    /// [`certification_country`](Self::certification_country).
    pub fn certification(mut self, cert: impl Into<String>) -> Self {
        self.certification = Some(cert.into());
        self
    }

    /// Minimum certification. Requires [`certification_country`](Self::certification_country).
    pub fn certification_gte(mut self, cert: impl Into<String>) -> Self {
        self.certification_gte = Some(cert.into());
        self
    }

    /// Maximum certification. Requires [`certification_country`](Self::certification_country).
    pub fn certification_lte(mut self, cert: impl Into<String>) -> Self {
        self.certification_lte = Some(cert.into());
        self
    }

    /// Include movies with a video entry.
    pub fn include_video(mut self, include: bool) -> Self {
        self.include_video = Some(include);
        self
    }

    /// Release date on or after (any release type).
    pub fn release_date_gte(mut self, date: NaiveDate) -> Self {
        self.release_date_gte = Some(date);
        self
    }

    /// Release date on or before (any release type).
    pub fn release_date_lte(mut self, date: NaiveDate) -> Self {
        self.release_date_lte = Some(date);
        self
    }

    /// Execute the discover query and return a single page of unified movies.
    ///
    /// # Errors
    ///
    /// Returns [`TmdbError::Api`] on non-2xx HTTP responses and
    /// [`TmdbError::Http`] on network failures.
    pub async fn execute(self) -> Result<Page<UnifiedMovie>, TmdbError> {
        self.client.acquire_rate_limit().await;
        let resp = self
            .client
            .inner()
            .discover_movie(
                self.certification.as_deref(),
                self.certification_gte.as_deref(),
                self.certification_lte.as_deref(),
                self.certification_country.as_deref(),
                self.include_adult,
                self.include_video,
                self.language.as_deref(),
                self.page,
                self.primary_release_date_gte.as_ref(),
                self.primary_release_date_lte.as_ref(),
                self.primary_release_year,
                self.region.as_deref(),
                self.release_date_gte.as_ref(),
                self.release_date_lte.as_ref(),
                self.sort_by.map(MovieSortBy::to_generated),
                self.vote_average_gte,
                self.vote_average_lte,
                self.vote_count_gte,
                self.vote_count_lte,
                self.watch_region.as_deref(),
                self.with_cast.as_deref(),
                self.with_companies.as_deref(),
                self.with_crew.as_deref(),
                self.with_genres.as_deref(),
                self.with_keywords.as_deref(),
                self.with_origin_country.as_deref(),
                self.with_original_language.as_deref(),
                self.with_people.as_deref(),
                self.with_release_type,
                self.with_runtime_gte,
                self.with_runtime_lte,
                self.with_watch_monetization_types.as_deref(),
                self.with_watch_providers.as_deref(),
                self.without_companies.as_deref(),
                self.without_genres.as_deref(),
                self.without_keywords.as_deref(),
                self.without_watch_providers.as_deref(),
                self.year,
            )
            .await?;
        let body = resp.into_inner();
        Ok(crate::providers::tmdb::client::offset_page(
            self.page,
            body.page,
            body.results,
            body.total_pages,
            body.total_results,
        )
        .map(Into::into))
    }
}
