use super::{CameoClient, CameoClientError};
use crate::unified::{
    external_ids::{ExternalId, ExternalIds, ExternalMatch},
    media_id::MediaId,
};

impl CameoClient {
    /// Get the external ids (IMDb, TVDB, MyAnimeList, …) of a movie, routed to
    /// the provider that minted `id`. Not cached.
    pub async fn movie_external_ids(&self, id: &MediaId) -> Result<ExternalIds, CameoClientError> {
        Ok(self
            .route(&self.external, id)?
            .movie_external_ids(id)
            .await?)
    }

    /// Get the external ids of a TV show, routed by `id` origin. Not cached.
    pub async fn tv_external_ids(&self, id: &MediaId) -> Result<ExternalIds, CameoClientError> {
        Ok(self.route(&self.external, id)?.tv_external_ids(id).await?)
    }

    /// Find titles matching a third-party id across every provider that
    /// accepts its source.
    ///
    /// Unlike the single-provider calls, this fans out: each registered
    /// [`ExternalIdProvider`](crate::unified::ExternalIdProvider) whose
    /// [`external_sources`](crate::unified::ExternalIdProvider::external_sources)
    /// lists `id.source` is queried concurrently, and the matches are merged in
    /// provider priority order with duplicates removed. A provider that fails
    /// is skipped (and logged at `warn`) as long as another succeeds; the
    /// first provider's error is returned only when every queried provider
    /// fails. Not cached.
    ///
    /// # Errors
    ///
    /// [`CameoClientError::Unsupported`] when no configured provider accepts
    /// `id.source`, [`CameoClientError::NotConfigured`] when nothing is
    /// configured, or [`CameoClientError::Provider`] when all queried
    /// providers fail.
    ///
    /// ```no_run
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # #[cfg(feature = "tmdb")]
    /// # {
    /// use cameo::providers::tmdb::TmdbConfig;
    /// use cameo::{CameoClient, ExternalId, MatchKind};
    ///
    /// let client = CameoClient::builder()
    ///     .with_tmdb(TmdbConfig::new("your-token"))
    ///     .build()?;
    /// for m in client.find_by_external_id(&ExternalId::imdb("tt0137523")).await? {
    ///     if m.kind == MatchKind::Movie {
    ///         let details = client.movie_details(&m.id).await?;
    ///         println!("{}", details.movie.title);
    ///     }
    /// }
    /// # }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn find_by_external_id(
        &self,
        id: &ExternalId,
    ) -> Result<Vec<ExternalMatch>, CameoClientError> {
        let providers: Vec<_> = self
            .external
            .iter()
            .filter(|p| p.external_sources().contains(&id.source))
            .collect();
        if providers.is_empty() {
            return Err(self.no_provider_error());
        }

        let outcomes =
            futures::future::join_all(providers.iter().map(|p| p.find_by_external_id(id))).await;

        let mut merged: Vec<ExternalMatch> = Vec::new();
        let mut first_error = None;
        let mut any_ok = false;
        for (provider, outcome) in providers.iter().zip(outcomes) {
            match outcome {
                Ok(matches) => {
                    any_ok = true;
                    for m in matches {
                        if !merged.contains(&m) {
                            merged.push(m);
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!(provider = provider.id(), error = %e, external_id = %id, "find_by_external_id failed");
                    first_error.get_or_insert(e);
                }
            }
        }
        match first_error {
            Some(e) if !any_ok => Err(e.into()),
            _ => Ok(merged),
        }
    }
}
