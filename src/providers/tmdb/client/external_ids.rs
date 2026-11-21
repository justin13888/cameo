//! External-id endpoints: a title's IMDb/TheTVDB/Wikidata ids, and lookup of
//! TMDB titles by one of them.

use super::{TmdbClient, requested_id};
use crate::{
    generated::tmdb::{self, types},
    providers::tmdb::error::TmdbError,
    unified::{
        conversions::native_id,
        external_ids::{ExternalId, ExternalIds, ExternalMatch, ExternalSource},
        media_id::MediaId,
    },
};

impl TmdbClient {
    /// Get the external ids (IMDb, Wikidata) of a movie.
    ///
    /// The returned [`ExternalIds`] also carries `tmdb: Some(movie_id)`. TMDB
    /// does not track TheTVDB or MyAnimeList ids for movies, so those stay
    /// `None`; empty strings TMDB sends for unknown ids are reported as `None`.
    ///
    /// # Errors
    ///
    /// [`TmdbError::Request`] when `movie_id` is zero or above `i32::MAX`
    /// (nothing is sent), plus the usual request errors.
    #[tracing::instrument(skip(self))]
    pub async fn movie_external_ids(&self, movie_id: u64) -> Result<ExternalIds, TmdbError> {
        // The requested id, not the body's (TMDB may send a null `id`).
        let wire_id = requested_id(movie_id)?;
        let body = self
            .execute(|| self.inner.movie_external_ids(wire_id))
            .await?;
        let mut ids = ExternalIds::new().with_tmdb(movie_id);
        ids.imdb = non_empty(body.imdb_id.into_option());
        // The spec leaves the movie `wikidata_id` untyped; it is a string or null.
        ids.wikidata = non_empty(
            body.wikidata_id
                .as_ref()
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned),
        );
        Ok(ids)
    }

    /// Get the external ids (IMDb, TheTVDB, Wikidata) of a TV series.
    ///
    /// The returned [`ExternalIds`] also carries `tmdb: Some(series_id)`. A
    /// TheTVDB id of `0` or below is reported as `None`.
    ///
    /// # Errors
    ///
    /// [`TmdbError::Request`] when `series_id` is zero or above `i32::MAX`
    /// (nothing is sent), plus the usual request errors.
    #[tracing::instrument(skip(self))]
    pub async fn tv_external_ids(&self, series_id: u64) -> Result<ExternalIds, TmdbError> {
        // The requested id, not the body's (TMDB may send a null `id`).
        let wire_id = requested_id(series_id)?;
        let body = self
            .execute(|| self.inner.tv_series_external_ids(wire_id))
            .await?;
        let mut ids = ExternalIds::new().with_tmdb(series_id);
        ids.imdb = non_empty(body.imdb_id.into_option());
        ids.tvdb = body.tvdb_id.into_option().and_then(native_id);
        ids.wikidata = non_empty(body.wikidata_id.into_option());
        Ok(ids)
    }

    /// Find TMDB movies and TV shows by an IMDb, TheTVDB, or Wikidata id
    /// (TMDB's `/find` endpoint).
    ///
    /// Only movie and TV-series results are returned (movies first); TMDB's
    /// person, season, and episode matches are dropped because they are not
    /// titles a [`MatchKind`](crate::unified::MatchKind) can describe.
    /// Results whose `id` is missing, malformed, zero, or negative are
    /// skipped. No match yields
    /// an empty `Vec`.
    ///
    /// # Errors
    ///
    /// [`TmdbError::UnsupportedSource`] for MyAnimeList ids (TMDB does not
    /// index them), plus the usual request errors.
    #[tracing::instrument(skip(self))]
    pub async fn find_by_external_id(
        &self,
        id: &ExternalId,
    ) -> Result<Vec<ExternalMatch>, TmdbError> {
        let source = match id.source {
            ExternalSource::Imdb => types::FindByIdExternalSource::ImdbId,
            ExternalSource::Tvdb => types::FindByIdExternalSource::TvdbId,
            ExternalSource::Wikidata => types::FindByIdExternalSource::WikidataId,
            other => return Err(TmdbError::UnsupportedSource(other)),
        };
        let params = tmdb::FindByIdParams {
            external_source: source,
            language: self.language(),
        };
        let body = self
            .execute(|| self.inner.find_by_id(id.id.as_str(), params.clone()))
            .await?;
        let movies = body
            .movie_results
            .as_option()
            .into_iter()
            .flatten()
            .filter_map(|m| m.id.into_option().and_then(native_id))
            .map(|n| ExternalMatch::movie(MediaId::tmdb(n)));
        // `tv_results` is untyped in the spec; only the `id` is needed.
        let shows = body
            .tv_results
            .as_option()
            .into_iter()
            .flatten()
            .filter_map(|v| v.get("id").and_then(serde_json::Value::as_i64))
            .filter_map(native_id)
            .map(|n| ExternalMatch::tv_show(MediaId::tmdb(n)));
        Ok(movies.chain(shows).collect())
    }
}

/// Treat an empty string (TMDB's placeholder for an unknown external id) as absent.
fn non_empty(value: Option<String>) -> Option<String> {
    value.filter(|s| !s.is_empty())
}
