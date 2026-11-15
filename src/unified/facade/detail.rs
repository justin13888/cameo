#[cfg(feature = "cache")]
use std::time::Duration;

use super::{CameoClient, CameoClientError};
#[cfg(feature = "cache")]
use crate::cache::{CacheKey, CacheTtlConfig, MediaType};
#[cfg(feature = "cache")]
use crate::unified::{date::PartialDate, models::SeriesStatus};
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
            let ttl = self.cache_ttl(|t| movie_details_ttl(t, &result));
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
            let ttl = self.cache_ttl(|t| tv_show_details_ttl(t, &result));
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

/// The age-aware TTL for a fetched movie, from its release date and status;
/// [`CacheTtlConfig::details`] when no age rule matches.
#[cfg(feature = "cache")]
fn movie_details_ttl(ttl: &CacheTtlConfig, details: &UnifiedMovieDetails) -> Duration {
    let release = details.movie.release_date.as_ref().map(iso_date);
    ttl.movie_details_ttl(release.as_deref(), details.status.as_deref())
}

/// The age-aware TTL for a fetched TV show, from its air dates, status and
/// production flag; [`CacheTtlConfig::details`] when no age rule matches.
#[cfg(feature = "cache")]
fn tv_show_details_ttl(ttl: &CacheTtlConfig, details: &UnifiedTvShowDetails) -> Duration {
    let first = details.show.first_air_date.as_ref().map(iso_date);
    let last = details.last_air_date.as_ref().map(iso_date);
    ttl.tv_show_details_ttl(
        first.as_deref(),
        last.as_deref(),
        details.status.as_ref().map(series_status_str),
        details.in_production.unwrap_or(false),
    )
}

/// `date` as `YYYY-MM-DD`, a year-only or year-month date counting as the
/// earliest day of its period.
#[cfg(feature = "cache")]
fn iso_date(date: &PartialDate) -> String {
    format!(
        "{:04}-{:02}-{:02}",
        date.year(),
        date.month().unwrap_or(1),
        date.day().unwrap_or(1)
    )
}

/// The status string [`CacheTtlConfig::tv_show_details_ttl`] recognises for
/// `status`; a provider value no variant covers is passed through verbatim.
#[cfg(feature = "cache")]
fn series_status_str(status: &SeriesStatus) -> &str {
    match status {
        SeriesStatus::ReturningSeries => "Returning Series",
        SeriesStatus::Planned => "Planned",
        SeriesStatus::InProduction => "In Production",
        SeriesStatus::Ended => "Ended",
        SeriesStatus::Cancelled => "Canceled",
        SeriesStatus::Pilot => "Pilot",
        SeriesStatus::Hiatus => "HIATUS",
        SeriesStatus::Other(other) => other,
    }
}
