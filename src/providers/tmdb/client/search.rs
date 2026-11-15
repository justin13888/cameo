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
    #[tracing::instrument(skip(self))]
    pub async fn search_movies(
        &self,
        query: &str,
        page: Option<u32>,
    ) -> Result<Page<UnifiedMovie>, TmdbError> {
        let params = tmdb::SearchMovieParams {
            include_adult: self.include_adult(),
            language: self.language(),
            page: Self::tmdb_page(page),
            region: self.region(),
            ..Default::default()
        };
        let body = self
            .execute(|| self.inner.search_movie(query, params.clone()))
            .await?;
        Ok(body.into_page(Self::tmdb_page(page))?.map(Into::into))
    }

    /// Search for TV shows by name.
    #[tracing::instrument(skip(self))]
    pub async fn search_tv_shows(
        &self,
        query: &str,
        page: Option<u32>,
    ) -> Result<Page<UnifiedTvShow>, TmdbError> {
        let params = tmdb::SearchTvParams {
            include_adult: self.include_adult(),
            language: self.language(),
            page: Self::tmdb_page(page),
            ..Default::default()
        };
        let body = self
            .execute(|| self.inner.search_tv(query, params.clone()))
            .await?;
        Ok(body.into_page(Self::tmdb_page(page))?.map(Into::into))
    }

    /// Search for people by name.
    #[tracing::instrument(skip(self))]
    pub async fn search_people(
        &self,
        query: &str,
        page: Option<u32>,
    ) -> Result<Page<UnifiedPerson>, TmdbError> {
        let params = tmdb::SearchPersonParams {
            include_adult: self.include_adult(),
            language: self.language(),
            page: Self::tmdb_page(page),
        };
        let body = self
            .execute(|| self.inner.search_person(query, params.clone()))
            .await?;
        Ok(body.into_page(Self::tmdb_page(page))?.map(Into::into))
    }

    /// Multi-search across movies, TV shows, and people.
    #[tracing::instrument(skip(self))]
    pub async fn search_multi(
        &self,
        query: &str,
        page: Option<u32>,
    ) -> Result<Page<UnifiedSearchResult>, TmdbError> {
        let params = tmdb::SearchMultiParams {
            include_adult: self.include_adult(),
            language: self.language(),
            page: Self::tmdb_page(page),
        };
        let body = self
            .execute(|| self.inner.search_multi(query, params.clone()))
            .await?;
        Ok(body
            .into_page(Self::tmdb_page(page))?
            .filter_map(multi_search_result))
    }
}
