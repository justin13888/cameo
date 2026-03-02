//! Caching layer for the cameo SDK.
//!
//! Provides a pluggable [`CacheBackend`] trait and a default [`SqliteCache`]
//! implementation. Use [`CacheTtlConfig`] to control how long different
//! response types are cached.

pub mod backend;
pub mod key;
pub mod sqlite;

pub use backend::{CacheBackend, CacheError};
pub use key::{CacheKey, MediaType};
pub use sqlite::SqliteCache;

use std::time::Duration;

use chrono::{NaiveDate, Utc};

/// TTL configuration for the cache layer.
///
/// Different response types have different staleness tolerances.
/// Age-aware TTL fields allow stable/old content to be cached longer while
/// keeping volatile or recently-released content fresher.
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
    /// TTL for volatile/dynamic data such as popularity scores and vote counts.
    /// Default: 1 hour.
    pub volatile_ttl: Duration,
}

impl Default for CacheTtlConfig {
    fn default() -> Self {
        Self {
            details: Duration::from_secs(24 * 3600),    // 24 hours (fallback)
            search: Duration::from_secs(3600),           // 1 hour
            discovery: Duration::from_secs(15 * 60),     // 15 minutes
            items: Duration::from_secs(6 * 3600),        // 6 hours
            old_content_threshold_days: 1095,            // ~3 years
            recent_content_threshold_days: 180,          // ~6 months
            old_content_details_ttl: Duration::from_secs(7 * 24 * 3600),  // 7 days
            recent_content_details_ttl: Duration::from_secs(6 * 3600),    // 6 hours
            active_content_details_ttl: Duration::from_secs(4 * 3600),    // 4 hours
            volatile_ttl: Duration::from_secs(3600),                       // 1 hour
        }
    }
}

impl CacheTtlConfig {
    fn parse_date(s: &str) -> Option<NaiveDate> {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()
    }

    fn days_since(date: NaiveDate) -> i64 {
        (Utc::now().date_naive() - date).num_days()
    }

    /// Compute the appropriate cache TTL for a movie detail entry.
    ///
    /// Selection logic (first match wins):
    /// 1. Status is "Released" **and** release date is older than
    ///    [`old_content_threshold_days`](CacheTtlConfig::old_content_threshold_days)
    ///    → [`old_content_details_ttl`](CacheTtlConfig::old_content_details_ttl)
    /// 2. Status indicates active production *or* release date is within
    ///    [`recent_content_threshold_days`](CacheTtlConfig::recent_content_threshold_days)
    ///    → [`recent_content_details_ttl`](CacheTtlConfig::recent_content_details_ttl)
    /// 3. Fallback → [`details`](CacheTtlConfig::details)
    pub fn movie_details_ttl(
        &self,
        release_date: Option<&str>,
        status: Option<&str>,
    ) -> Duration {
        let release_days = release_date.and_then(Self::parse_date).map(Self::days_since);

        let is_released = status.map(|s| s == "Released").unwrap_or(false);
        let is_old = release_days
            .map(|d| d > self.old_content_threshold_days as i64)
            .unwrap_or(false);

        if is_released && is_old {
            return self.old_content_details_ttl;
        }

        let is_in_production = matches!(
            status,
            Some("In Production") | Some("Post Production") | Some("Planned") | Some("Rumored")
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
    pub fn tv_show_details_ttl(
        &self,
        first_air_date: Option<&str>,
        last_air_date: Option<&str>,
        status: Option<&str>,
        in_production: bool,
    ) -> Duration {
        let last_air_days = last_air_date.and_then(Self::parse_date).map(Self::days_since);
        let recently_aired = last_air_days.map(|d| d <= 90).unwrap_or(false);

        if in_production || recently_aired {
            return self.active_content_details_ttl;
        }

        let first_air_days =
            first_air_date.and_then(Self::parse_date).map(Self::days_since);
        let is_ended = matches!(status, Some("Ended") | Some("Canceled"));
        let is_old = first_air_days
            .map(|d| d > self.old_content_threshold_days as i64)
            .unwrap_or(false);

        if is_ended && is_old {
            return self.old_content_details_ttl;
        }

        let is_returning = matches!(
            status,
            Some("Returning Series") | Some("In Production") | Some("Planned")
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
