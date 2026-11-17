use super::{CameoClient, CameoClientError};
#[cfg(feature = "cache")]
use crate::cache::{CacheKey, MediaType};
use crate::unified::{
    media_id::MediaId,
    models::{UnifiedMovieDetails, UnifiedPersonDetails, UnifiedTvShowDetails},
};

impl CameoClient {
    /// Get detailed information about a movie, routed to the provider that minted `id`.
    pub async fn movie_details(
        &self,
        id: &MediaId,
    ) -> Result<UnifiedMovieDetails, CameoClientError> {
        #[cfg(feature = "cache")]
        let key = CacheKey::Detail {
            media_type: MediaType::Movie,
            provider_id: id.to_string(),
        };
        #[cfg(feature = "cache")]
        if let Some(hit) = self.cache_get::<UnifiedMovieDetails>(&key).await {
            return Ok(hit);
        }
        let result = self.route(&self.detail, id)?.movie_details(id).await?;
        #[cfg(feature = "cache")]
        {
            let ttl = self.cache_ttl(|t| t.movie_details_ttl(&result));
            self.cache_put(key, &result, ttl).await;
            self.cache_item(&result.movie).await;
        }
        Ok(result)
    }

    /// Get detailed information about a TV show, routed by `id` origin.
    pub async fn tv_show_details(
        &self,
        id: &MediaId,
    ) -> Result<UnifiedTvShowDetails, CameoClientError> {
        #[cfg(feature = "cache")]
        let key = CacheKey::Detail {
            media_type: MediaType::TvShow,
            provider_id: id.to_string(),
        };
        #[cfg(feature = "cache")]
        if let Some(hit) = self.cache_get::<UnifiedTvShowDetails>(&key).await {
            return Ok(hit);
        }
        let result = self.route(&self.detail, id)?.tv_show_details(id).await?;
        #[cfg(feature = "cache")]
        {
            let ttl = self.cache_ttl(|t| t.tv_show_details_ttl(&result));
            self.cache_put(key, &result, ttl).await;
            self.cache_item(&result.show).await;
        }
        Ok(result)
    }

    /// Get detailed information about a person, routed by `id` origin.
    pub async fn person_details(
        &self,
        id: &MediaId,
    ) -> Result<UnifiedPersonDetails, CameoClientError> {
        #[cfg(feature = "cache")]
        let key = CacheKey::Detail {
            media_type: MediaType::Person,
            provider_id: id.to_string(),
        };
        #[cfg(feature = "cache")]
        if let Some(hit) = self.cache_get::<UnifiedPersonDetails>(&key).await {
            return Ok(hit);
        }
        let result = self.route(&self.detail, id)?.person_details(id).await?;
        #[cfg(feature = "cache")]
        {
            self.cache_put(key, &result, self.cache_ttl(|t| t.details))
                .await;
            self.cache_item(&result.person).await;
        }
        Ok(result)
    }
}
