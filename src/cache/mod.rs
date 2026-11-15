//! Caching layer for the cameo SDK.
//!
//! Provides a pluggable [`CacheBackend`] trait and a default [`SqliteCache`]
//! implementation. Use [`CacheTtlConfig`] to control how long different
//! response types are cached.

pub mod backend;
pub mod key;
pub mod sqlite;

use std::time::Duration;

pub use backend::{CacheBackend, CacheError};
use chrono::{NaiveDate, Utc};
pub use key::{CacheKey, MediaType};
pub use sqlite::SqliteCache;

use crate::unified::{PartialDate, UnifiedMovieDetails, UnifiedTvShowDetails};

/// TTL configuration for the cache layer.
///
/// Different response types have different staleness tolerances.
/// Age-aware TTL fields allow stable/old content to be cached longer while
/// keeping volatile or recently-released content fresher.
///
/// Start from [`Default`] and override fields with the `with_*` setters:
///
/// ```
/// use std::time::Duration;
/// use cameo::cache::CacheTtlConfig;
///
/// let ttl = CacheTtlConfig::default()
///     .with_search(Duration::from_secs(300))
///     .with_old_content_threshold_days(730);
/// assert_eq!(ttl.search, Duration::from_secs(300));
/// assert_eq!(ttl.old_content_threshold_days, 730);
/// ```
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct CacheTtlConfig {
    /// Fallback TTL for detail responses when no age policy matches.
    pub details: Duration,
    /// TTL for search result pages.
    pub search: Duration,
    /// TTL for discovery/listing result pages.
    pub discovery: Duration,
    /// TTL for individual items indexed from list results.
    pub items: Duration,

    // ── Age-aware policy fields ──────────────────────────────────────────────
    /// Content is considered "old" when its release/air date is at least this
    /// many days in the past (default: 1095 = ~3 years).
    pub old_content_threshold_days: u32,
    /// Content is considered "recent" when its release/air date is within this
    /// many days of today (default: 180 = ~6 months).
    pub recent_content_threshold_days: u32,
    /// TTL applied to old, stable content (e.g. a movie released 5 years ago
    /// with status "Released"). Default: 7 days.
    pub old_content_details_ttl: Duration,
    /// TTL applied to recently-released or in-production content (e.g. a movie
    /// released within the past 6 months). Default: 6 hours.
    pub recent_content_details_ttl: Duration,
    /// TTL applied to actively-airing TV shows (`in_production = true` or
    /// `last_air_date` within the past 90 days). Default: 4 hours.
    pub active_content_details_ttl: Duration,
}

impl Default for CacheTtlConfig {
    fn default() -> Self {
        Self {
            details: Duration::from_secs(24 * 3600), // 24 hours (fallback)
            search: Duration::from_secs(3600),       // 1 hour
            discovery: Duration::from_secs(15 * 60), // 15 minutes
            items: Duration::from_secs(6 * 3600),    // 6 hours
            old_content_threshold_days: 1095,        // ~3 years
            recent_content_threshold_days: 180,      // ~6 months
            old_content_details_ttl: Duration::from_secs(7 * 24 * 3600), // 7 days
            recent_content_details_ttl: Duration::from_secs(6 * 3600), // 6 hours
            active_content_details_ttl: Duration::from_secs(4 * 3600), // 4 hours
        }
    }
}

impl CacheTtlConfig {
    /// Set the fallback TTL for detail responses ([`details`](Self::details)).
    pub fn with_details(mut self, value: Duration) -> Self {
        self.details = value;
        self
    }

    /// Set the TTL for search result pages ([`search`](Self::search)).
    pub fn with_search(mut self, value: Duration) -> Self {
        self.search = value;
        self
    }

    /// Set the TTL for discovery/listing result pages
    /// ([`discovery`](Self::discovery)).
    pub fn with_discovery(mut self, value: Duration) -> Self {
        self.discovery = value;
        self
    }

    /// Set the TTL for items indexed from list results ([`items`](Self::items)).
    pub fn with_items(mut self, value: Duration) -> Self {
        self.items = value;
        self
    }

    /// Set the age in days past which content counts as old
    /// ([`old_content_threshold_days`](Self::old_content_threshold_days)).
    pub fn with_old_content_threshold_days(mut self, value: u32) -> Self {
        self.old_content_threshold_days = value;
        self
    }

    /// Set the age in days within which content counts as recent
    /// ([`recent_content_threshold_days`](Self::recent_content_threshold_days)).
    pub fn with_recent_content_threshold_days(mut self, value: u32) -> Self {
        self.recent_content_threshold_days = value;
        self
    }

    /// Set the TTL for old, stable content
    /// ([`old_content_details_ttl`](Self::old_content_details_ttl)).
    pub fn with_old_content_details_ttl(mut self, value: Duration) -> Self {
        self.old_content_details_ttl = value;
        self
    }

    /// Set the TTL for recent or in-production content
    /// ([`recent_content_details_ttl`](Self::recent_content_details_ttl)).
    pub fn with_recent_content_details_ttl(mut self, value: Duration) -> Self {
        self.recent_content_details_ttl = value;
        self
    }

    /// Set the TTL for actively-airing TV shows
    /// ([`active_content_details_ttl`](Self::active_content_details_ttl)).
    pub fn with_active_content_details_ttl(mut self, value: Duration) -> Self {
        self.active_content_details_ttl = value;
        self
    }

    /// Days elapsed since the earliest day of `date`'s period.
    ///
    /// A year-only date counts as January 1st and a year-month date as the
    /// 1st of that month. A full date naming an impossible day (e.g.
    /// February 30th) counts as the 1st of its month. Returns `None` only when
    /// the year is outside the range [`NaiveDate`] can represent.
    fn days_since(date: PartialDate) -> Option<i64> {
        let month = u32::from(date.month().unwrap_or(1));
        let day = u32::from(date.day().unwrap_or(1));
        let earliest = NaiveDate::from_ymd_opt(date.year(), month, day)
            .or_else(|| NaiveDate::from_ymd_opt(date.year(), month, 1))?;
        Some((Utc::now().date_naive() - earliest).num_days())
    }

    /// Compute the appropriate cache TTL for a movie detail entry.
    ///
    /// The release date is the base movie's
    /// [`release_date`](crate::unified::models::UnifiedMovie::release_date); a
    /// partial date counts as the earliest day of its period (a year-only
    /// `2022` counts as 2022-01-01). The status is
    /// [`UnifiedMovieDetails::status`], falling back to the base movie's
    /// [`status`](crate::unified::models::UnifiedMovie::status).
    ///
    /// Selection logic (first match wins):
    /// 1. Status is "Released" **and** release date is older than
    ///    [`old_content_threshold_days`](CacheTtlConfig::old_content_threshold_days)
    ///    → [`old_content_details_ttl`](CacheTtlConfig::old_content_details_ttl)
    /// 2. Status indicates active production *or* release date is within
    ///    [`recent_content_threshold_days`](CacheTtlConfig::recent_content_threshold_days)
    ///    → [`recent_content_details_ttl`](CacheTtlConfig::recent_content_details_ttl)
    /// 3. Fallback → [`details`](CacheTtlConfig::details)
    pub fn movie_details_ttl(&self, details: &UnifiedMovieDetails) -> Duration {
        let status = details
            .status
            .as_deref()
            .or(details.movie.status.as_deref());
        let release_days = details.movie.release_date.and_then(Self::days_since);

        // Recognize both TMDB ("Released") and AniList ("FINISHED") statuses so
        // the age-aware policy applies to both providers, not just TMDB.
        let is_released = matches!(status, Some("Released") | Some("FINISHED"));
        let is_old = release_days
            .map(|d| d > self.old_content_threshold_days as i64)
            .unwrap_or(false);

        if is_released && is_old {
            return self.old_content_details_ttl;
        }

        let is_in_production = matches!(
            status,
            Some("In Production")
                | Some("Post Production")
                | Some("Planned")
                | Some("Rumored")
                | Some("RELEASING")
                | Some("NOT_YET_RELEASED")
        );
        let is_recent = release_days
            .map(|d| d <= self.recent_content_threshold_days as i64)
            .unwrap_or(false);

        if is_in_production || is_recent {
            return self.recent_content_details_ttl;
        }

        self.details
    }

    /// Compute the appropriate cache TTL for a TV show detail entry.
    ///
    /// Selection logic (first match wins):
    /// 1. `in_production` is `true` *or* `last_air_date` is within the past 90
    ///    days → [`active_content_details_ttl`](CacheTtlConfig::active_content_details_ttl)
    /// 2. Status is "Ended" or "Canceled" **and** `first_air_date` is older
    ///    than [`old_content_threshold_days`](CacheTtlConfig::old_content_threshold_days)
    ///    → [`old_content_details_ttl`](CacheTtlConfig::old_content_details_ttl)
    /// 3. Status indicates continuing/planned *or* `first_air_date` is within
    ///    [`recent_content_threshold_days`](CacheTtlConfig::recent_content_threshold_days)
    ///    → [`recent_content_details_ttl`](CacheTtlConfig::recent_content_details_ttl)
    /// 4. Fallback → [`details`](CacheTtlConfig::details)
    ///
    /// The dates are [`UnifiedTvShowDetails::last_air_date`] and the base
    /// show's
    /// [`first_air_date`](crate::unified::models::UnifiedTvShow::first_air_date);
    /// a partial date counts as the earliest day of its period. The status is
    /// [`UnifiedTvShowDetails::status`], falling back to the base show's
    /// [`status`](crate::unified::models::UnifiedTvShow::status). An unknown
    /// [`in_production`](UnifiedTvShowDetails::in_production) counts as
    /// `false`, so it falls through to the date- and status-based rules.
    pub fn tv_show_details_ttl(&self, details: &UnifiedTvShowDetails) -> Duration {
        let status = details.status.as_deref().or(details.show.status.as_deref());
        let in_production = details.in_production.unwrap_or(false);
        let last_air_days = details.last_air_date.and_then(Self::days_since);
        let recently_aired = last_air_days.map(|d| d <= 90).unwrap_or(false);

        if in_production || recently_aired {
            return self.active_content_details_ttl;
        }

        let first_air_days = details.show.first_air_date.and_then(Self::days_since);
        let is_ended = matches!(
            status,
            Some("Ended") | Some("Canceled") | Some("FINISHED") | Some("CANCELLED")
        );
        let is_old = first_air_days
            .map(|d| d > self.old_content_threshold_days as i64)
            .unwrap_or(false);

        if is_ended && is_old {
            return self.old_content_details_ttl;
        }

        let is_returning = matches!(
            status,
            Some("Returning Series")
                | Some("In Production")
                | Some("Planned")
                | Some("RELEASING")
                | Some("NOT_YET_RELEASED")
        );
        let is_recent = first_air_days
            .map(|d| d <= self.recent_content_threshold_days as i64)
            .unwrap_or(false);

        if is_returning || is_recent {
            return self.recent_content_details_ttl;
        }

        self.details
    }
}
