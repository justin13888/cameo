//! Details endpoints: movie, TV series, and person details, credits, images,
//! genres, and certifications.

use super::{TmdbClient, details_id, requested_id};
use crate::{
    generated::tmdb::{self, types},
    providers::tmdb::{
        convert::certifications,
        error::TmdbError,
        models::{TmdbCertification, TmdbCredits, TmdbImages},
    },
    unified::{
        genre::Genre,
        models::{UnifiedMovieDetails, UnifiedPersonDetails, UnifiedTvShowDetails},
    },
};

impl TmdbClient {
    // ── Details ──

    /// Get detailed information about a movie.
    #[tracing::instrument(skip(self))]
    pub async fn movie_details(&self, movie_id: i32) -> Result<UnifiedMovieDetails, TmdbError> {
        let params = tmdb::MovieDetailsParams {
            language: self.language(),
            ..Default::default()
        };
        let mut body = self
            .execute(|| self.inner.movie_details(movie_id, params.clone()))
            .await?;
        // TMDB may send a null/missing or invalid id; fall back to the requested one.
        details_id(&mut body.id, movie_id);
        Ok(body.into())
    }

    /// Get detailed information about a movie with appended responses.
    #[tracing::instrument(skip(self))]
    pub async fn movie_details_with_append(
        &self,
        movie_id: i32,
        append: &str,
    ) -> Result<UnifiedMovieDetails, TmdbError> {
        let params = tmdb::MovieDetailsParams {
            append_to_response: Some(append.to_owned()),
            language: self.language(),
        };
        let mut body = self
            .execute(|| self.inner.movie_details(movie_id, params.clone()))
            .await?;
        details_id(&mut body.id, movie_id);
        Ok(body.into())
    }

    /// Get detailed information about a TV series.
    #[tracing::instrument(skip(self))]
    pub async fn tv_series_details(
        &self,
        series_id: i32,
    ) -> Result<UnifiedTvShowDetails, TmdbError> {
        let params = tmdb::TvSeriesDetailsParams {
            language: self.language(),
            ..Default::default()
        };
        let mut body = self
            .execute(|| self.inner.tv_series_details(series_id, params.clone()))
            .await?;
        details_id(&mut body.id, series_id);
        Ok(body.into())
    }

    /// Get detailed information about a person.
    #[tracing::instrument(skip(self))]
    pub async fn person_details(&self, person_id: i32) -> Result<UnifiedPersonDetails, TmdbError> {
        let params = tmdb::PersonDetailsParams {
            language: self.language(),
            ..Default::default()
        };
        let mut body = self
            .execute(|| self.inner.person_details(person_id, params.clone()))
            .await?;
        details_id(&mut body.id, person_id);
        Ok(body.into())
    }

    // ── Credits ──

    /// Get the cast and crew for a movie.
    #[tracing::instrument(skip(self))]
    pub async fn movie_credits(&self, movie_id: i32) -> Result<TmdbCredits, TmdbError> {
        self.movie_credits_response(movie_id).await.map(Into::into)
    }

    /// Get the cast and crew for a TV series.
    ///
    /// Comes from `/tv/{id}/aggregate_credits`, which lists each person once.
    /// [`TmdbCredits`] keeps one entry per person, holding only the first of
    /// their characters and the first of their jobs.
    #[tracing::instrument(skip(self))]
    pub async fn tv_series_credits(&self, series_id: i32) -> Result<TmdbCredits, TmdbError> {
        self.tv_series_aggregate_credits_response(series_id)
            .await
            .map(Into::into)
    }

    /// Fetch `/movie/{id}/credits` as TMDB returned it.
    pub(crate) async fn movie_credits_response(
        &self,
        movie_id: i32,
    ) -> Result<types::MovieCreditsResponse, TmdbError> {
        let params = tmdb::MovieCreditsParams {
            language: self.language(),
        };
        self.execute(|| self.inner.movie_credits(movie_id, params.clone()))
            .await
    }

    /// Fetch `/tv/{id}/aggregate_credits` as TMDB returned it, with every
    /// character and job of each person.
    pub(crate) async fn tv_series_aggregate_credits_response(
        &self,
        series_id: i32,
    ) -> Result<types::TvSeriesAggregateCreditsResponse, TmdbError> {
        let params = tmdb::TvSeriesAggregateCreditsParams {
            language: self.language(),
        };
        self.execute(|| {
            self.inner
                .tv_series_aggregate_credits(series_id, params.clone())
        })
        .await
    }

    // ── Certifications ──

    /// Get a movie's certification (content rating) per country, from TMDB's
    /// `/movie/{id}/release_dates`.
    ///
    /// A country lists one release per type (premiere, theatrical, digital,
    /// ...). Each country appears at most once: its theatrical release's
    /// certification when that is non-empty, otherwise the first non-empty
    /// one. Countries with no certification are left out, so a title nobody
    /// rated yields an empty `Vec`.
    ///
    /// # Errors
    ///
    /// [`TmdbError::Request`] when `movie_id` is zero or negative (nothing is
    /// sent), plus the usual request errors.
    #[tracing::instrument(skip(self))]
    pub async fn movie_certifications(
        &self,
        movie_id: i32,
    ) -> Result<Vec<TmdbCertification>, TmdbError> {
        requested_id(movie_id)?;
        let body = self
            .execute(|| self.inner.movie_release_dates(movie_id))
            .await?;
        Ok(certifications::movie_certifications(body))
    }

    /// Get a TV series' certification (content rating) per country, from
    /// TMDB's `/tv/{id}/content_ratings`.
    ///
    /// Entries with no country code or an empty rating are left out, so a
    /// series nobody rated yields an empty `Vec`.
    ///
    /// # Errors
    ///
    /// [`TmdbError::Request`] when `series_id` is zero or negative (nothing is
    /// sent), plus the usual request errors.
    #[tracing::instrument(skip(self))]
    pub async fn tv_series_certifications(
        &self,
        series_id: i32,
    ) -> Result<Vec<TmdbCertification>, TmdbError> {
        requested_id(series_id)?;
        let body = self
            .execute(|| self.inner.tv_series_content_ratings(series_id))
            .await?;
        Ok(certifications::tv_series_certifications(body))
    }

    // ── Genres ──

    /// Get the list of official movie genres.
    #[tracing::instrument(skip(self))]
    pub async fn movie_genres(&self) -> Result<Vec<Genre>, TmdbError> {
        let params = tmdb::GenreMovieListParams {
            language: self.language(),
        };
        let resp = self
            .execute(|| self.inner.genre_movie_list(params.clone()))
            .await?;
        Ok(resp
            .genres
            .into_iter()
            .flatten()
            .filter_map(|g| g.id.map(Genre::from_tmdb_id))
            .collect())
    }

    /// Get the list of official TV show genres.
    #[tracing::instrument(skip(self))]
    pub async fn tv_genres(&self) -> Result<Vec<Genre>, TmdbError> {
        let params = tmdb::GenreTvListParams {
            language: self.language(),
        };
        let resp = self
            .execute(|| self.inner.genre_tv_list(params.clone()))
            .await?;
        Ok(resp
            .genres
            .into_iter()
            .flatten()
            .filter_map(|g| g.id.map(Genre::from_tmdb_id))
            .collect())
    }

    // ── Images ──

    /// Get images for a movie.
    #[tracing::instrument(skip(self))]
    pub async fn movie_images(&self, movie_id: i32) -> Result<TmdbImages, TmdbError> {
        let params = tmdb::MovieImagesParams {
            language: self.language(),
            ..Default::default()
        };
        self.execute(|| self.inner.movie_images(movie_id, params.clone()))
            .await
            .map(Into::into)
    }
}
