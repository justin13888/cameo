use chrono::NaiveDate;

use crate::core::pagination::PaginatedResponse;
use crate::generated::tmdb::types::{DiscoverTvResponseResultsItem, DiscoverTvSortBy};
use crate::providers::tmdb::error::TmdbError;
use crate::providers::tmdb::TmdbClient;

/// Builder for discovering TV shows with flexible filters.
///
/// # Example
///
/// ```no_run
/// # async fn example(client: &cameo::providers::tmdb::TmdbClient) -> Result<(), cameo::providers::tmdb::TmdbError> {
/// let results = client
///     .discover_tv()
///     .sort_by(cameo::generated::tmdb::types::DiscoverTvSortBy::PopularityDesc)
///     .with_genres("10765")
///     .first_air_date_year(2024)
///     .execute()
///     .await?;
/// # Ok(())
/// # }
/// ```
pub struct DiscoverTvBuilder<'a> {
    client: &'a TmdbClient,
    air_date_gte: Option<NaiveDate>,
    air_date_lte: Option<NaiveDate>,
    first_air_date_gte: Option<NaiveDate>,
    first_air_date_lte: Option<NaiveDate>,
    first_air_date_year: Option<i32>,
    include_adult: Option<bool>,
    include_null_first_air_dates: Option<bool>,
    language: Option<String>,
    page: Option<i32>,
    screened_theatrically: Option<bool>,
    sort_by: Option<DiscoverTvSortBy>,
    timezone: Option<String>,
    vote_average_gte: Option<f32>,
    vote_average_lte: Option<f32>,
    vote_count_gte: Option<f32>,
    vote_count_lte: Option<f32>,
    watch_region: Option<String>,
    with_companies: Option<String>,
    with_genres: Option<String>,
    with_keywords: Option<String>,
    with_networks: Option<i32>,
    with_origin_country: Option<String>,
    with_original_language: Option<String>,
    with_runtime_gte: Option<i32>,
    with_runtime_lte: Option<i32>,
    with_status: Option<String>,
    with_type: Option<String>,
    with_watch_monetization_types: Option<String>,
    with_watch_providers: Option<String>,
    without_companies: Option<String>,
    without_genres: Option<String>,
    without_keywords: Option<String>,
    without_watch_providers: Option<String>,
}

impl<'a> DiscoverTvBuilder<'a> {
    pub(crate) fn new(client: &'a TmdbClient) -> Self {
        Self {
            client,
            air_date_gte: None,
            air_date_lte: None,
            first_air_date_gte: None,
            first_air_date_lte: None,
            first_air_date_year: None,
            include_adult: client.config().include_adult,
            include_null_first_air_dates: None,
            language: client.config().language.clone(),
            page: None,
            screened_theatrically: None,
            sort_by: None,
            timezone: None,
            vote_average_gte: None,
            vote_average_lte: None,
            vote_count_gte: None,
            vote_count_lte: None,
            watch_region: None,
            with_companies: None,
            with_genres: None,
            with_keywords: None,
            with_networks: None,
            with_origin_country: None,
            with_original_language: None,
            with_runtime_gte: None,
            with_runtime_lte: None,
            with_status: None,
            with_type: None,
            with_watch_monetization_types: None,
            with_watch_providers: None,
            without_companies: None,
            without_genres: None,
            without_keywords: None,
            without_watch_providers: None,
        }
    }

    /// Set the sort order.
    pub fn sort_by(mut self, sort_by: DiscoverTvSortBy) -> Self {
        self.sort_by = Some(sort_by);
        self
    }

    /// Filter by genre IDs (comma-separated).
    pub fn with_genres(mut self, genres: impl Into<String>) -> Self {
        self.with_genres = Some(genres.into());
        self
    }

    /// Exclude genre IDs (comma-separated).
    pub fn without_genres(mut self, genres: impl Into<String>) -> Self {
        self.without_genres = Some(genres.into());
        self
    }

    /// Filter by first air date year.
    pub fn first_air_date_year(mut self, year: i32) -> Self {
        self.first_air_date_year = Some(year);
        self
    }

    /// Filter by first air date on or after.
    pub fn first_air_date_gte(mut self, date: NaiveDate) -> Self {
        self.first_air_date_gte = Some(date);
        self
    }

    /// Filter by first air date on or before.
    pub fn first_air_date_lte(mut self, date: NaiveDate) -> Self {
        self.first_air_date_lte = Some(date);
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

    /// Filter by network ID.
    pub fn with_networks(mut self, network_id: i32) -> Self {
        self.with_networks = Some(network_id);
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

    /// Filter by original language (ISO 639-1).
    pub fn with_original_language(mut self, lang: impl Into<String>) -> Self {
        self.with_original_language = Some(lang.into());
        self
    }

    /// Filter by origin country (ISO 3166-1).
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

    /// Execute the discover query and return a single page of results.
    pub async fn execute(
        self,
    ) -> Result<PaginatedResponse<DiscoverTvResponseResultsItem>, TmdbError> {
        let resp = self
            .client
            .inner()
            .discover_tv(
                self.air_date_gte.as_ref(),
                self.air_date_lte.as_ref(),
                self.first_air_date_gte.as_ref(),
                self.first_air_date_lte.as_ref(),
                self.first_air_date_year,
                self.include_adult,
                self.include_null_first_air_dates,
                self.language.as_deref(),
                self.page,
                self.screened_theatrically,
                self.sort_by,
                self.timezone.as_deref(),
                self.vote_average_gte,
                self.vote_average_lte,
                self.vote_count_gte,
                self.vote_count_lte,
                self.watch_region.as_deref(),
                self.with_companies.as_deref(),
                self.with_genres.as_deref(),
                self.with_keywords.as_deref(),
                self.with_networks,
                self.with_origin_country.as_deref(),
                self.with_original_language.as_deref(),
                self.with_runtime_gte,
                self.with_runtime_lte,
                self.with_status.as_deref(),
                self.with_type.as_deref(),
                self.with_watch_monetization_types.as_deref(),
                self.with_watch_providers.as_deref(),
                self.without_companies.as_deref(),
                self.without_genres.as_deref(),
                self.without_keywords.as_deref(),
                self.without_watch_providers.as_deref(),
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
