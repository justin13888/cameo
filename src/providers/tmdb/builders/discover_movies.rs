use chrono::NaiveDate;

use crate::{
    core::pagination::PaginatedResponse,
    generated::tmdb::types::{DiscoverMovieResponseResultsItem, DiscoverMovieSortBy},
    providers::tmdb::{TmdbClient, error::TmdbError},
};

/// Builder for discovering movies with flexible filters.
///
/// # Example
///
/// ```no_run
/// # async fn example(client: &cameo::providers::tmdb::TmdbClient) -> Result<(), cameo::providers::tmdb::TmdbError> {
/// let results = client
///     .discover_movies()
///     .sort_by(cameo::generated::tmdb::types::DiscoverMovieSortBy::PopularityDesc)
///     .with_genres("28")
///     .primary_release_year(2024)
///     .execute()
///     .await?;
/// # Ok(())
/// # }
/// ```
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
    sort_by: Option<DiscoverMovieSortBy>,
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

    /// Set the sort order.
    pub fn sort_by(mut self, sort_by: DiscoverMovieSortBy) -> Self {
        self.sort_by = Some(sort_by);
        self
    }

    /// Filter by genre IDs.
    ///
    /// Accepts a comma-separated list of TMDB genre IDs (e.g. `"28,12"` for
    /// Action + Adventure). AND semantics: only items matching **all** listed
    /// genres are returned.
    pub fn with_genres(mut self, genres: impl Into<String>) -> Self {
        self.with_genres = Some(genres.into());
        self
    }

    /// Exclude genre IDs.
    ///
    /// Accepts a comma-separated list of TMDB genre IDs. Items matching any
    /// of the listed genres are excluded from results.
    pub fn without_genres(mut self, genres: impl Into<String>) -> Self {
        self.without_genres = Some(genres.into());
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

    /// Minimum vote average.
    pub fn vote_average_gte(mut self, min: f32) -> Self {
        self.vote_average_gte = Some(min);
        self
    }

    /// Maximum vote average.
    pub fn vote_average_lte(mut self, max: f32) -> Self {
        self.vote_average_lte = Some(max);
        self
    }

    /// Minimum vote count.
    pub fn vote_count_gte(mut self, min: f32) -> Self {
        self.vote_count_gte = Some(min);
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

    /// Filter by cast member IDs (comma-separated).
    pub fn with_cast(mut self, cast: impl Into<String>) -> Self {
        self.with_cast = Some(cast.into());
        self
    }

    /// Filter by crew member IDs (comma-separated).
    pub fn with_crew(mut self, crew: impl Into<String>) -> Self {
        self.with_crew = Some(crew.into());
        self
    }

    /// Filter by keyword IDs (comma-separated).
    pub fn with_keywords(mut self, keywords: impl Into<String>) -> Self {
        self.with_keywords = Some(keywords.into());
        self
    }

    /// Filter by company IDs (comma-separated).
    pub fn with_companies(mut self, companies: impl Into<String>) -> Self {
        self.with_companies = Some(companies.into());
        self
    }

    /// Filter by original language.
    ///
    /// Accepts an ISO 639-1 language code (e.g. `"en"`, `"ja"`, `"fr"`).
    pub fn with_original_language(mut self, lang: impl Into<String>) -> Self {
        self.with_original_language = Some(lang.into());
        self
    }

    /// Filter by origin country.
    ///
    /// Accepts an ISO 3166-1 alpha-2 country code (e.g. `"US"`, `"JP"`, `"GB"`).
    pub fn with_origin_country(mut self, country: impl Into<String>) -> Self {
        self.with_origin_country = Some(country.into());
        self
    }

    /// Minimum runtime in minutes.
    pub fn with_runtime_gte(mut self, min: i32) -> Self {
        self.with_runtime_gte = Some(min);
        self
    }

    /// Maximum runtime in minutes.
    pub fn with_runtime_lte(mut self, max: i32) -> Self {
        self.with_runtime_lte = Some(max);
        self
    }

    /// Set the page number.
    pub fn page(mut self, page: u32) -> Self {
        self.page = Some(page as i32);
        self
    }

    /// Include adult content.
    pub fn include_adult(mut self, include: bool) -> Self {
        self.include_adult = Some(include);
        self
    }

    /// Certification country.
    ///
    /// Accepts an ISO 3166-1 alpha-2 country code. Required when filtering by
    /// certification. Example: `"US"`.
    pub fn certification_country(mut self, country: impl Into<String>) -> Self {
        self.certification_country = Some(country.into());
        self
    }

    /// Filter by exact certification rating.
    ///
    /// Example values: `"G"`, `"PG"`, `"PG-13"`, `"R"`. Requires
    /// [`certification_country`](Self::certification_country) to also be set.
    pub fn certification(mut self, cert: impl Into<String>) -> Self {
        self.certification = Some(cert.into());
        self
    }

    /// Execute the discover query and return a single page of results.
    ///
    /// # Errors
    ///
    /// Returns [`TmdbError::RateLimitExceeded`] if all concurrency permits are
    /// occupied and a `rate_limit_timeout` is configured. Returns
    /// [`TmdbError::Api`] on non-2xx HTTP responses. Returns
    /// [`TmdbError::Http`] on network failures.
    pub async fn execute(
        self,
    ) -> Result<PaginatedResponse<DiscoverMovieResponseResultsItem>, TmdbError> {
        let _permit = self.client.acquire_rate_limit_permit().await?;
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
                self.sort_by,
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
        Ok(PaginatedResponse {
            page: body.page as u32,
            results: body.results,
            total_pages: body.total_pages as u32,
            total_results: body.total_results as u32,
        })
    }
}
