use chrono::NaiveDate;

use super::params::{self, TvShowType, TvStatus};
use crate::{
    core::pagination::Page,
    providers::tmdb::{TmdbClient, builders::TvSortBy, error::TmdbError},
    unified::models::UnifiedTvShow,
};

/// Builder for discovering TV shows with flexible filters.
///
/// The ~15 filters shared with
/// [`DiscoverMoviesBuilder`](super::DiscoverMoviesBuilder) come from the shared
/// setter core in [`super::params`]; the setters below are
/// TV-specific. Provider ID filters take `&[u64]` slices and coded filters take
/// typed enums.
///
/// # Example
///
/// ```no_run
/// # async fn example(client: &cameo::providers::tmdb::TmdbClient) -> Result<(), cameo::providers::tmdb::TmdbError> {
/// use cameo::providers::tmdb::builders::TvSortBy;
/// let results = client
///     .discover_tv()
///     .sort_by(TvSortBy::PopularityDesc)
///     .with_genres(&[10765]) // Sci-Fi & Fantasy
///     .first_air_date_year(2024)
///     .execute()
///     .await?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
#[must_use = "a discover query does nothing until .execute() is awaited"]
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
    sort_by: Option<TvSortBy>,
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

    // The ~15 filters common to both discover builders, defined once.
    params::common_discover_setters!();

    /// Set the sort order.
    pub fn sort_by(mut self, sort_by: TvSortBy) -> Self {
        self.sort_by = Some(sort_by);
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

    /// Air date on or after.
    pub fn air_date_gte(mut self, date: NaiveDate) -> Self {
        self.air_date_gte = Some(date);
        self
    }

    /// Air date on or before.
    pub fn air_date_lte(mut self, date: NaiveDate) -> Self {
        self.air_date_lte = Some(date);
        self
    }

    /// Include shows with no first air date.
    pub fn include_null_first_air_dates(mut self, include: bool) -> Self {
        self.include_null_first_air_dates = Some(include);
        self
    }

    /// Include shows that have been screened theatrically.
    pub fn screened_theatrically(mut self, screened: bool) -> Self {
        self.screened_theatrically = Some(screened);
        self
    }

    /// Timezone for air date filtering (e.g. `"America/New_York"`).
    pub fn timezone(mut self, tz: impl Into<String>) -> Self {
        self.timezone = Some(tz.into());
        self
    }

    /// Filter by network ID.
    pub fn with_networks(mut self, network_id: u64) -> Self {
        self.with_networks = Some(network_id.min(i32::MAX as u64) as i32);
        self
    }

    /// Filter by origin country (ISO 3166-1 alpha-2, e.g. `"US"`).
    pub fn with_origin_country(mut self, country: impl Into<String>) -> Self {
        self.with_origin_country = Some(country.into());
        self
    }

    /// Filter by show status.
    pub fn with_status(mut self, status: TvStatus) -> Self {
        self.with_status = Some(status.code().to_string());
        self
    }

    /// Filter by show type.
    pub fn with_type(mut self, show_type: TvShowType) -> Self {
        self.with_type = Some(show_type.code().to_string());
        self
    }

    /// Execute the discover query and return a single page of unified shows.
    ///
    /// # Errors
    ///
    /// Returns [`TmdbError::Api`] on non-2xx HTTP responses and
    /// [`TmdbError::Http`] on network failures.
    pub async fn execute(self) -> Result<Page<UnifiedTvShow>, TmdbError> {
        self.client.acquire_rate_limit().await;
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
                self.sort_by.map(TvSortBy::to_generated),
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
        Ok(Page::offset(
            body.page as u32,
            body.results,
            body.total_pages as u32,
            body.total_results as u32,
        )
        .map(Into::into))
    }
}
