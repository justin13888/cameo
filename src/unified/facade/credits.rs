use super::{CameoClient, CameoClientError};
use crate::unified::{media_id::MediaId, models::UnifiedCredits};

impl CameoClient {
    /// Get the cast and crew of a movie, routed to the provider that minted
    /// `id`. Not cached.
    ///
    /// # Errors
    ///
    /// [`CameoClientError::Unsupported`] when no configured credits provider
    /// minted `id`, or [`CameoClientError::Provider`] when the lookup fails.
    ///
    /// ```no_run
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # #[cfg(feature = "tmdb")]
    /// # {
    /// use cameo::providers::tmdb::TmdbConfig;
    /// use cameo::{CameoClient, MediaId};
    ///
    /// let client = CameoClient::builder()
    ///     .with_tmdb(TmdbConfig::new("your-token"))
    ///     .build()?;
    /// let credits = client.movie_credits(&MediaId::tmdb(550)).await?;
    /// for cast in credits.cast.iter().take(3) {
    ///     println!("{} as {:?}", cast.name, cast.character);
    /// }
    /// # }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn movie_credits(&self, id: &MediaId) -> Result<UnifiedCredits, CameoClientError> {
        Ok(self.route(&self.credits, id)?.movie_credits(id).await?)
    }

    /// Get the cast and crew of a TV show, routed by `id` origin. Not cached.
    ///
    /// # Errors
    ///
    /// As [`movie_credits`](Self::movie_credits).
    pub async fn tv_show_credits(&self, id: &MediaId) -> Result<UnifiedCredits, CameoClientError> {
        Ok(self.route(&self.credits, id)?.tv_show_credits(id).await?)
    }
}
