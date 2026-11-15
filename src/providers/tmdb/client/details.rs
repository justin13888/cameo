//! Details endpoints: movie, TV series, and person details, credits, images,
//! genres, and certifications.

use super::{TmdbClient, details_id, requested_id, wire_int};
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
    ///
    /// # Errors
    ///
    /// [`TmdbError::Request`] when `movie_id` is above `i32::MAX` (nothing is
    /// sent), [`TmdbError::Api`] with status `404` when no movie has this id,
    /// plus the [usual request errors](TmdbClient#errors).
    #[tracing::instrument(skip(self))]
    pub async fn movie_details(&self, movie_id: u64) -> Result<UnifiedMovieDetails, TmdbError> {
        let movie_id = wire_int(movie_id, "id")?;
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
    ///
    /// `append` is TMDB's comma-separated `append_to_response` list. Including
    /// `release_dates` fills [`UnifiedMovieDetails::certifications`] with one
    /// certification per country, picked as
    /// [`movie_certifications`](Self::movie_certifications) picks them; without
    /// it that field stays empty. Other appended blocks are not read.
    ///
    /// # Errors
    ///
    /// As [`movie_details`](Self::movie_details).
    #[tracing::instrument(skip(self))]
    pub async fn movie_details_with_append(
        &self,
        movie_id: u64,
        append: &str,
    ) -> Result<UnifiedMovieDetails, TmdbError> {
        let movie_id = wire_int(movie_id, "id")?;
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
    ///
    /// # Errors
    ///
    /// [`TmdbError::Request`] when `series_id` is above `i32::MAX` (nothing is
    /// sent), [`TmdbError::Api`] with status `404` when no series has this id,
    /// plus the [usual request errors](TmdbClient#errors).
    #[tracing::instrument(skip(self))]
    pub async fn tv_show_details(&self, series_id: u64) -> Result<UnifiedTvShowDetails, TmdbError> {
        let series_id = wire_int(series_id, "id")?;
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

    /// Get detailed information about a TV series with appended responses.
    ///
    /// `append` is TMDB's comma-separated `append_to_response` list. Including
    /// `content_ratings` fills [`UnifiedTvShowDetails::certifications`] with
    /// the series' content ratings, kept as
    /// [`tv_show_certifications`](Self::tv_show_certifications) keeps them;
    /// without it that field stays empty. Other appended blocks are not read.
    ///
    /// # Errors
    ///
    /// As [`tv_show_details`](Self::tv_show_details).
    #[tracing::instrument(skip(self))]
    pub async fn tv_show_details_with_append(
        &self,
        series_id: u64,
        append: &str,
    ) -> Result<UnifiedTvShowDetails, TmdbError> {
        let series_id = wire_int(series_id, "id")?;
        let params = tmdb::TvSeriesDetailsParams {
            append_to_response: Some(append.to_owned()),
            language: self.language(),
        };
        let mut body = self
            .execute(|| self.inner.tv_series_details(series_id, params.clone()))
            .await?;
        details_id(&mut body.id, series_id);
        Ok(body.into())
    }

    /// Get detailed information about a person.
    ///
    /// # Errors
    ///
    /// [`TmdbError::Request`] when `person_id` is above `i32::MAX` (nothing is
    /// sent), [`TmdbError::Api`] with status `404` when no person has this id,
    /// plus the [usual request errors](TmdbClient#errors).
    #[tracing::instrument(skip(self))]
    pub async fn person_details(&self, person_id: u64) -> Result<UnifiedPersonDetails, TmdbError> {
        let person_id = wire_int(person_id, "id")?;
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
    ///
    /// # Errors
    ///
    /// As [`movie_details`](Self::movie_details).
    #[tracing::instrument(skip(self))]
    pub async fn movie_credits(&self, movie_id: u64) -> Result<TmdbCredits, TmdbError> {
        self.movie_credits_response(movie_id).await.map(Into::into)
    }

    /// Get the cast and crew for a TV series.
    ///
    /// Comes from `/tv/{id}/aggregate_credits`, which lists each person once.
    /// [`TmdbCredits`] keeps one entry per person, holding only the first of
    /// their characters and the first of their jobs.
    ///
    /// # Errors
    ///
    /// As [`tv_show_details`](Self::tv_show_details).
    #[tracing::instrument(skip(self))]
    pub async fn tv_show_credits(&self, series_id: u64) -> Result<TmdbCredits, TmdbError> {
        self.tv_series_aggregate_credits_response(series_id)
            .await
            .map(Into::into)
    }

    /// Fetch `/movie/{id}/credits` as TMDB returned it.
    pub(crate) async fn movie_credits_response(
        &self,
        movie_id: u64,
    ) -> Result<types::MovieCreditsResponse, TmdbError> {
        let movie_id = wire_int(movie_id, "id")?;
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
        series_id: u64,
    ) -> Result<types::TvSeriesAggregateCreditsResponse, TmdbError> {
        let series_id = wire_int(series_id, "id")?;
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
    /// [`TmdbError::Request`] when `movie_id` is zero or above `i32::MAX`
    /// (nothing is sent), plus the usual request errors.
    #[tracing::instrument(skip(self))]
    pub async fn movie_certifications(
        &self,
        movie_id: u64,
    ) -> Result<Vec<TmdbCertification>, TmdbError> {
        let movie_id = requested_id(movie_id)?;
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
    /// [`TmdbError::Request`] when `series_id` is zero or above `i32::MAX`
    /// (nothing is sent), plus the usual request errors.
    #[tracing::instrument(skip(self))]
    pub async fn tv_show_certifications(
        &self,
        series_id: u64,
    ) -> Result<Vec<TmdbCertification>, TmdbError> {
        let series_id = requested_id(series_id)?;
        let body = self
            .execute(|| self.inner.tv_series_content_ratings(series_id))
            .await?;
        Ok(certifications::tv_series_certifications(body))
    }

    // ── Genres ──

    /// Get the list of official movie genres.
    ///
    /// Entries without an id are skipped.
    ///
    /// # Errors
    ///
    /// The [usual request errors](TmdbClient#errors).
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
    ///
    /// Entries without an id are skipped.
    ///
    /// # Errors
    ///
    /// The [usual request errors](TmdbClient#errors).
    #[tracing::instrument(skip(self))]
    pub async fn tv_show_genres(&self) -> Result<Vec<Genre>, TmdbError> {
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
    ///
    /// # Errors
    ///
    /// As [`movie_details`](Self::movie_details).
    #[tracing::instrument(skip(self))]
    pub async fn movie_images(&self, movie_id: u64) -> Result<TmdbImages, TmdbError> {
        let movie_id = wire_int(movie_id, "id")?;
        let params = tmdb::MovieImagesParams {
            language: self.language(),
            ..Default::default()
        };
        self.execute(|| self.inner.movie_images(movie_id, params.clone()))
            .await
            .map(Into::into)
    }
}
