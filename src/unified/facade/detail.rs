use async_trait::async_trait;

use super::{CameoClient, CameoClientError};
#[cfg(feature = "cache")]
use crate::cache::{CacheKey, MediaType};
use crate::unified::{
    models::{UnifiedMovieDetails, UnifiedPersonDetails, UnifiedTvShowDetails},
    traits::DetailProvider,
};

#[async_trait]
impl DetailProvider for CameoClient {
    type Error = CameoClientError;

    async fn movie_details(&self, id: i32) -> Result<UnifiedMovieDetails, CameoClientError> {
        #[cfg(feature = "tmdb")]
        if let Some(client) = &self.tmdb {
            #[cfg(feature = "cache")]
            let provider_id = format!("tmdb:{id}");

            #[cfg(feature = "cache")]
            {
                let detail_key = CacheKey::Detail {
                    media_type: MediaType::Movie,
                    provider_id: provider_id.clone(),
                };
                if let Some(cache) = self.cache.as_ref()
                    && let Some(cached) = cache.get::<UnifiedMovieDetails>(&detail_key).await
                {
                    tracing::debug!(id, "cache hit: movie_details (tmdb)");
                    return Ok(cached);
                }
                if self.cache.is_some() {
                    tracing::debug!(id, "cache miss: movie_details (tmdb)");
                }
            }

            tracing::debug!(id, "dispatching movie_details to tmdb");
            let details: UnifiedMovieDetails = client.movie_details(id).await?.into();

            #[cfg(feature = "cache")]
            if let Some(cache) = self.cache.as_ref() {
                let detail_ttl = cache.ttl.movie_details_ttl(
                    details.movie.release_date.as_deref(),
                    details.status.as_deref(),
                );
                let detail_key = CacheKey::Detail {
                    media_type: MediaType::Movie,
                    provider_id: provider_id.clone(),
                };
                cache.set(detail_key, &details, detail_ttl).await;
                let item_key = CacheKey::Item {
                    media_type: MediaType::Movie,
                    provider_id,
                };
                cache.set(item_key, &details.movie, cache.ttl.items).await;
            }

            return Ok(details);
        }

        #[cfg(feature = "anilist")]
        if let Some(client) = &self.anilist {
            #[cfg(feature = "cache")]
            let provider_id = format!("anilist:{id}");

            #[cfg(feature = "cache")]
            {
                let detail_key = CacheKey::Detail {
                    media_type: MediaType::Movie,
                    provider_id: provider_id.clone(),
                };
                if let Some(cache) = self.cache.as_ref()
                    && let Some(cached) = cache.get::<UnifiedMovieDetails>(&detail_key).await
                {
                    tracing::debug!(id, "cache hit: movie_details (anilist)");
                    return Ok(cached);
                }
                if self.cache.is_some() {
                    tracing::debug!(id, "cache miss: movie_details (anilist)");
                }
            }

            tracing::debug!(id, "dispatching movie_details to anilist");
            let details = client.movie_details(id).await?;

            #[cfg(feature = "cache")]
            if let Some(cache) = self.cache.as_ref() {
                let detail_ttl = cache.ttl.movie_details_ttl(
                    details.movie.release_date.as_deref(),
                    details.status.as_deref(),
                );
                let detail_key = CacheKey::Detail {
                    media_type: MediaType::Movie,
                    provider_id: provider_id.clone(),
                };
                cache.set(detail_key, &details, detail_ttl).await;
                let item_key = CacheKey::Item {
                    media_type: MediaType::Movie,
                    provider_id,
                };
                cache.set(item_key, &details.movie, cache.ttl.items).await;
            }

            return Ok(details);
        }

        Err(CameoClientError::NoProviders)
    }

    async fn tv_show_details(&self, id: i32) -> Result<UnifiedTvShowDetails, CameoClientError> {
        #[cfg(feature = "tmdb")]
        if let Some(client) = &self.tmdb {
            #[cfg(feature = "cache")]
            let provider_id = format!("tmdb:{id}");

            #[cfg(feature = "cache")]
            {
                let detail_key = CacheKey::Detail {
                    media_type: MediaType::TvShow,
                    provider_id: provider_id.clone(),
                };
                if let Some(cache) = self.cache.as_ref()
                    && let Some(cached) = cache.get::<UnifiedTvShowDetails>(&detail_key).await
                {
                    tracing::debug!(id, "cache hit: tv_show_details (tmdb)");
                    return Ok(cached);
                }
                if self.cache.is_some() {
                    tracing::debug!(id, "cache miss: tv_show_details (tmdb)");
                }
            }

            tracing::debug!(id, "dispatching tv_show_details to tmdb");
            let details: UnifiedTvShowDetails = client.tv_series_details(id).await?.into();

            #[cfg(feature = "cache")]
            if let Some(cache) = self.cache.as_ref() {
                let detail_ttl = cache.ttl.tv_show_details_ttl(
                    details.show.first_air_date.as_deref(),
                    details.last_air_date.as_deref(),
                    details.status.as_deref(),
                    details.in_production,
                );
                let detail_key = CacheKey::Detail {
                    media_type: MediaType::TvShow,
                    provider_id: provider_id.clone(),
                };
                cache.set(detail_key, &details, detail_ttl).await;
                let item_key = CacheKey::Item {
                    media_type: MediaType::TvShow,
                    provider_id,
                };
                cache.set(item_key, &details.show, cache.ttl.items).await;
            }

            return Ok(details);
        }

        #[cfg(feature = "anilist")]
        if let Some(client) = &self.anilist {
            #[cfg(feature = "cache")]
            let provider_id = format!("anilist:{id}");

            #[cfg(feature = "cache")]
            {
                let detail_key = CacheKey::Detail {
                    media_type: MediaType::TvShow,
                    provider_id: provider_id.clone(),
                };
                if let Some(cache) = self.cache.as_ref()
                    && let Some(cached) = cache.get::<UnifiedTvShowDetails>(&detail_key).await
                {
                    tracing::debug!(id, "cache hit: tv_show_details (anilist)");
                    return Ok(cached);
                }
                if self.cache.is_some() {
                    tracing::debug!(id, "cache miss: tv_show_details (anilist)");
                }
            }

            tracing::debug!(id, "dispatching tv_show_details to anilist");
            let details = client.tv_show_details(id).await?;

            #[cfg(feature = "cache")]
            if let Some(cache) = self.cache.as_ref() {
                let detail_ttl = cache.ttl.tv_show_details_ttl(
                    details.show.first_air_date.as_deref(),
                    details.last_air_date.as_deref(),
                    details.status.as_deref(),
                    details.in_production,
                );
                let detail_key = CacheKey::Detail {
                    media_type: MediaType::TvShow,
                    provider_id: provider_id.clone(),
                };
                cache.set(detail_key, &details, detail_ttl).await;
                let item_key = CacheKey::Item {
                    media_type: MediaType::TvShow,
                    provider_id,
                };
                cache.set(item_key, &details.show, cache.ttl.items).await;
            }

            return Ok(details);
        }

        Err(CameoClientError::NoProviders)
    }

    async fn person_details(&self, id: i32) -> Result<UnifiedPersonDetails, CameoClientError> {
        #[cfg(feature = "tmdb")]
        if let Some(client) = &self.tmdb {
            #[cfg(feature = "cache")]
            let provider_id = format!("tmdb:{id}");

            #[cfg(feature = "cache")]
            {
                let detail_key = CacheKey::Detail {
                    media_type: MediaType::Person,
                    provider_id: provider_id.clone(),
                };
                if let Some(cache) = self.cache.as_ref()
                    && let Some(cached) = cache.get::<UnifiedPersonDetails>(&detail_key).await
                {
                    tracing::debug!(id, "cache hit: person_details (tmdb)");
                    return Ok(cached);
                }
                if self.cache.is_some() {
                    tracing::debug!(id, "cache miss: person_details (tmdb)");
                }
            }

            tracing::debug!(id, "dispatching person_details to tmdb");
            let details: UnifiedPersonDetails = client.person_details(id).await?.into();

            #[cfg(feature = "cache")]
            if let Some(cache) = self.cache.as_ref() {
                let detail_key = CacheKey::Detail {
                    media_type: MediaType::Person,
                    provider_id: provider_id.clone(),
                };
                cache.set(detail_key, &details, cache.ttl.details).await;
                let item_key = CacheKey::Item {
                    media_type: MediaType::Person,
                    provider_id,
                };
                cache.set(item_key, &details.person, cache.ttl.items).await;
            }

            return Ok(details);
        }

        #[cfg(feature = "anilist")]
        if let Some(client) = &self.anilist {
            #[cfg(feature = "cache")]
            let provider_id = format!("anilist:staff:{id}");

            #[cfg(feature = "cache")]
            {
                let detail_key = CacheKey::Detail {
                    media_type: MediaType::Person,
                    provider_id: provider_id.clone(),
                };
                if let Some(cache) = self.cache.as_ref()
                    && let Some(cached) = cache.get::<UnifiedPersonDetails>(&detail_key).await
                {
                    tracing::debug!(id, "cache hit: person_details (anilist)");
                    return Ok(cached);
                }
                if self.cache.is_some() {
                    tracing::debug!(id, "cache miss: person_details (anilist)");
                }
            }

            tracing::debug!(id, "dispatching person_details to anilist");
            let details = client.person_details(id).await?;

            #[cfg(feature = "cache")]
            if let Some(cache) = self.cache.as_ref() {
                let detail_key = CacheKey::Detail {
                    media_type: MediaType::Person,
                    provider_id: provider_id.clone(),
                };
                cache.set(detail_key, &details, cache.ttl.details).await;
                let item_key = CacheKey::Item {
                    media_type: MediaType::Person,
                    provider_id,
                };
                cache.set(item_key, &details.person, cache.ttl.items).await;
            }

            return Ok(details);
        }

        Err(CameoClientError::NoProviders)
    }
}
