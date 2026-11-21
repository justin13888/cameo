//! Search endpoints: movies, TV shows, people, and multi-search.

use super::TmdbClient;
use crate::{
    core::pagination::Page,
    generated::tmdb,
    providers::tmdb::{
        convert::{TmdbPaged, person::multi_search_result},
        error::TmdbError,
    },
    unified::models::{UnifiedMovie, UnifiedPerson, UnifiedSearchResult, UnifiedTvShow},
};

impl TmdbClient {
    /// Search for movies by title.
    ///
    /// `page` is clamped as [`TmdbClient`'s pages](TmdbClient#pages) are. No
    /// match yields an empty page.
    ///
    /// # Errors
    ///
    /// The [usual request errors](TmdbClient#errors).
    #[tracing::instrument(skip(self))]
    pub async fn search_movies(
        &self,
        query: &str,
        page: Option<u32>,
    ) -> Result<Page<UnifiedMovie>, TmdbError> {
        let params = tmdb::SearchMovieParams {
            query: query.to_owned(),
            include_adult: self.include_adult(),
            language: self.language(),
            primary_release_year: None,
            page: Self::tmdb_page(page),
            region: self.region(),
            year: None,
        };
        let body = self
            .execute(|| self.inner.search_movie(params.clone()))
            .await?;
        Ok(body.into_page(Self::tmdb_page(page))?.map(Into::into))
    }

    /// Search for TV shows by name.
    ///
    /// `page` is clamped as [`TmdbClient`'s pages](TmdbClient#pages) are. No
    /// match yields an empty page.
    ///
    /// # Errors
    ///
    /// The [usual request errors](TmdbClient#errors).
    #[tracing::instrument(skip(self))]
    pub async fn search_tv_shows(
        &self,
        query: &str,
        page: Option<u32>,
    ) -> Result<Page<UnifiedTvShow>, TmdbError> {
        let params = tmdb::SearchTvParams {
            query: query.to_owned(),
            first_air_date_year: None,
            include_adult: self.include_adult(),
            language: self.language(),
            page: Self::tmdb_page(page),
            year: None,
        };
        let body = self
            .execute(|| self.inner.search_tv(params.clone()))
            .await?;
        Ok(body.into_page(Self::tmdb_page(page))?.map(Into::into))
    }

    /// Search for people by name.
    ///
    /// `page` is clamped as [`TmdbClient`'s pages](TmdbClient#pages) are. No
    /// match yields an empty page.
    ///
    /// # Errors
    ///
    /// The [usual request errors](TmdbClient#errors).
    #[tracing::instrument(skip(self))]
    pub async fn search_people(
        &self,
        query: &str,
        page: Option<u32>,
    ) -> Result<Page<UnifiedPerson>, TmdbError> {
        let params = tmdb::SearchPersonParams {
            query: query.to_owned(),
            include_adult: self.include_adult(),
            language: self.language(),
            page: Self::tmdb_page(page),
        };
        let body = self
            .execute(|| self.inner.search_person(params.clone()))
            .await?;
        Ok(body.into_page(Self::tmdb_page(page))?.map(Into::into))
    }

    /// Multi-search across movies, TV shows, and people.
    ///
    /// `page` is clamped as [`TmdbClient`'s pages](TmdbClient#pages) are. An
    /// item whose media type is missing or unknown, or whose id is not a valid
    /// TMDB id, is omitted, so a page may hold fewer items than TMDB sent
    /// while its pagination totals keep describing TMDB's unfiltered results.
    ///
    /// # Errors
    ///
    /// The [usual request errors](TmdbClient#errors).
    #[tracing::instrument(skip(self))]
    pub async fn search_multi(
        &self,
        query: &str,
        page: Option<u32>,
    ) -> Result<Page<UnifiedSearchResult>, TmdbError> {
        let params = tmdb::SearchMultiParams {
            query: query.to_owned(),
            include_adult: self.include_adult(),
            language: self.language(),
            page: Self::tmdb_page(page),
        };
        let body = self
            .execute(|| self.inner.search_multi(params.clone()))
            .await?;
        Ok(body
            .into_page(Self::tmdb_page(page))?
            .filter_map(multi_search_result))
    }
}
