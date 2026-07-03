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
            self.cache_put(key, &result, self.cache_details_ttl()).await;
            self.cache_put(
                CacheKey::Item {
                    media_type: MediaType::Movie,
                    provider_id: result.movie.provider_id.to_string(),
                },
                &result.movie,
                self.cache_items_ttl(),
            )
            .await;
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
            self.cache_put(key, &result, self.cache_details_ttl()).await;
            self.cache_put(
                CacheKey::Item {
                    media_type: MediaType::TvShow,
                    provider_id: result.show.provider_id.to_string(),
                },
                &result.show,
                self.cache_items_ttl(),
            )
            .await;
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
            self.cache_put(key, &result, self.cache_details_ttl()).await;
            self.cache_put(
                CacheKey::Item {
                    media_type: MediaType::Person,
                    provider_id: result.person.provider_id.to_string(),
                },
                &result.person,
                self.cache_items_ttl(),
            )
            .await;
        }
        Ok(result)
    }
}
