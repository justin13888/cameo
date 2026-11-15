#![cfg(feature = "cache")]

use std::time::Duration;

use cameo::{
    MediaId, PartialDate, UnifiedMovie, UnifiedMovieDetails, UnifiedTvShow, UnifiedTvShowDetails,
    cache::{CacheBackend, CacheKey, CacheTtlConfig, MediaType, SqliteCache},
};

fn make_cache() -> SqliteCache {
    SqliteCache::in_memory().expect("in-memory SQLite cache")
}

#[tokio::test]
async fn get_returns_none_for_missing_key() {
    let cache = make_cache();
    let key = CacheKey::Detail {
        media_type: MediaType::Movie,
        provider_id: "tmdb:550".to_string(),
    };
    let result = cache.get(&key).await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn set_then_get_returns_value() {
    let cache = make_cache();
    let key = CacheKey::Detail {
        media_type: MediaType::Movie,
        provider_id: "tmdb:550".to_string(),
    };
    let value = serde_json::json!({ "title": "Fight Club", "id": 550 });

    cache
        .set(key.clone(), value.clone(), Duration::from_secs(60))
        .await
        .unwrap();
    let result = cache.get(&key).await.unwrap();
    assert_eq!(result, Some(value));
}

#[tokio::test]
async fn set_overwrites_existing_value() {
    let cache = make_cache();
    let key = CacheKey::Item {
        media_type: MediaType::TvShow,
        provider_id: "tmdb:1396".to_string(),
    };
    cache
        .set(
            key.clone(),
            serde_json::json!({ "v": 1 }),
            Duration::from_secs(60),
        )
        .await
        .unwrap();
    cache
        .set(
            key.clone(),
            serde_json::json!({ "v": 2 }),
            Duration::from_secs(60),
        )
        .await
        .unwrap();
    let result = cache.get(&key).await.unwrap().unwrap();
    assert_eq!(result["v"], 2);
}

#[tokio::test]
async fn invalidate_removes_entry() {
    let cache = make_cache();
    let key = CacheKey::Search {
        provider: "tmdb".to_string(),
        media_type: Some(MediaType::Movie),
        query: "dune".to_string(),
        page: 1,
    };
    cache
        .set(
            key.clone(),
            serde_json::json!([1, 2, 3]),
            Duration::from_secs(60),
        )
        .await
        .unwrap();
    cache.invalidate(&key).await.unwrap();
    let result = cache.get(&key).await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn clear_removes_all_entries() {
    let cache = make_cache();
    let k1 = CacheKey::Detail {
        media_type: MediaType::Movie,
        provider_id: "tmdb:1".to_string(),
    };
    let k2 = CacheKey::Detail {
        media_type: MediaType::Movie,
        provider_id: "tmdb:2".to_string(),
    };
    cache
        .set(k1.clone(), serde_json::json!(1), Duration::from_secs(60))
        .await
        .unwrap();
    cache
        .set(k2.clone(), serde_json::json!(2), Duration::from_secs(60))
        .await
        .unwrap();

    cache.clear().await.unwrap();

    assert!(cache.get(&k1).await.unwrap().is_none());
    assert!(cache.get(&k2).await.unwrap().is_none());
}

#[tokio::test]
async fn expired_entry_returns_none() {
    let cache = make_cache();
    let key = CacheKey::Discovery {
        provider: "tmdb".to_string(),
        endpoint: "trending_movies".to_string(),
        page: 1,
    };
    // Set with 1-second TTL
    cache
        .set(
            key.clone(),
            serde_json::json!({ "expired": true }),
            Duration::from_secs(1),
        )
        .await
        .unwrap();
    // Sleep past expiry
    tokio::time::sleep(Duration::from_secs(2)).await;
    let result = cache.get(&key).await.unwrap();
    assert!(result.is_none(), "expired entry should not be returned");
}

#[tokio::test]
async fn multiple_key_types_are_independent() {
    let cache = make_cache();
    let detail_key = CacheKey::Detail {
        media_type: MediaType::Movie,
        provider_id: "tmdb:550".to_string(),
    };
    let item_key = CacheKey::Item {
        media_type: MediaType::Movie,
        provider_id: "tmdb:550".to_string(),
    };
    // Same provider_id but different key_type
    cache
        .set(
            detail_key.clone(),
            serde_json::json!({ "type": "detail" }),
            Duration::from_secs(60),
        )
        .await
        .unwrap();
    cache
        .set(
            item_key.clone(),
            serde_json::json!({ "type": "item" }),
            Duration::from_secs(60),
        )
        .await
        .unwrap();

    assert_eq!(
        cache.get(&detail_key).await.unwrap().unwrap()["type"],
        "detail"
    );
    assert_eq!(cache.get(&item_key).await.unwrap().unwrap()["type"], "item");
}

// ── Key serialization ──

#[test]
fn cache_key_type_discriminants() {
    assert_eq!(
        CacheKey::Detail {
            media_type: MediaType::Movie,
            provider_id: "tmdb:550".to_string()
        }
        .key_type(),
        "detail"
    );
    assert_eq!(
        CacheKey::Item {
            media_type: MediaType::Movie,
            provider_id: "tmdb:550".to_string()
        }
        .key_type(),
        "item"
    );
    assert_eq!(
        CacheKey::Search {
            provider: "tmdb".to_string(),
            media_type: None,
            query: "q".to_string(),
            page: 1
        }
        .key_type(),
        "search"
    );
    assert_eq!(
        CacheKey::Discovery {
            provider: "tmdb".to_string(),
            endpoint: "e".to_string(),
            page: 1
        }
        .key_type(),
        "discovery"
    );
}

#[test]
fn search_key_normalizes_whitespace_and_case() {
    let k1 = CacheKey::Search {
        provider: "tmdb".to_string(),
        media_type: Some(MediaType::Movie),
        query: "  Dune  ".to_string(),
        page: 1,
    };
    let k2 = CacheKey::Search {
        provider: "tmdb".to_string(),
        media_type: Some(MediaType::Movie),
        query: "dune".to_string(),
        page: 1,
    };
    assert_eq!(k1.key_id(), k2.key_id());
}

// ── Age-aware TTL policy ───────────────────────────────────────────────────────

fn days_ago(n: i64) -> PartialDate {
    use chrono::{Duration, Utc};
    PartialDate::from(Utc::now().date_naive() - Duration::days(n))
}

fn movie(release_date: Option<PartialDate>, status: Option<&str>) -> UnifiedMovieDetails {
    let mut base = UnifiedMovie::new(MediaId::tmdb(550), "Fight Club");
    if let Some(date) = release_date {
        base = base.with_release_date(date);
    }
    let mut details = UnifiedMovieDetails::new(base);
    if let Some(status) = status {
        details = details.with_status(status);
    }
    details
}

fn tv(
    first_air_date: Option<PartialDate>,
    last_air_date: Option<PartialDate>,
    status: Option<&str>,
    in_production: Option<bool>,
) -> UnifiedTvShowDetails {
    let mut base = UnifiedTvShow::new(MediaId::tmdb(1399), "Game of Thrones");
    if let Some(date) = first_air_date {
        base = base.with_first_air_date(date);
    }
    let mut details = UnifiedTvShowDetails::new(base);
    if let Some(date) = last_air_date {
        details = details.with_last_air_date(date);
    }
    if let Some(status) = status {
        details = details.with_status(status);
    }
    if let Some(flag) = in_production {
        details = details.with_in_production(flag);
    }
    details
}

#[test]
fn movie_old_released_gets_long_ttl() {
    let cfg = CacheTtlConfig::default();
    let release = days_ago(1200); // ~3.3 years ago
    let ttl = cfg.movie_details_ttl(&movie(Some(release), Some("Released")));
    assert_eq!(
        ttl, cfg.old_content_details_ttl,
        "old released movie should use old_content_details_ttl"
    );
}

#[test]
fn movie_recent_gets_medium_ttl() {
    let cfg = CacheTtlConfig::default();
    let release = days_ago(60); // 2 months ago
    let ttl = cfg.movie_details_ttl(&movie(Some(release), Some("Released")));
    assert_eq!(
        ttl, cfg.recent_content_details_ttl,
        "recently released movie should use recent_content_details_ttl"
    );
}

#[test]
fn movie_in_production_gets_medium_ttl() {
    let cfg = CacheTtlConfig::default();
    let ttl = cfg.movie_details_ttl(&movie(None, Some("In Production")));
    assert_eq!(
        ttl, cfg.recent_content_details_ttl,
        "in-production movie should use recent_content_details_ttl"
    );
}

#[test]
fn movie_post_production_gets_medium_ttl() {
    let cfg = CacheTtlConfig::default();
    let ttl = cfg.movie_details_ttl(&movie(Some(days_ago(30)), Some("Post Production")));
    assert_eq!(ttl, cfg.recent_content_details_ttl);
}

#[test]
fn movie_released_recently_gets_medium_even_if_status_missing() {
    let cfg = CacheTtlConfig::default();
    let release = days_ago(90);
    let ttl = cfg.movie_details_ttl(&movie(Some(release), None));
    assert_eq!(ttl, cfg.recent_content_details_ttl);
}

#[test]
fn movie_released_mid_range_falls_back_to_details_ttl() {
    let cfg = CacheTtlConfig::default();
    // 500 days ago: not "old" (>1095) and not "recent" (<180), status "Released"
    let release = days_ago(500);
    let ttl = cfg.movie_details_ttl(&movie(Some(release), Some("Released")));
    assert_eq!(
        ttl, cfg.details,
        "mid-range released movie should fall back to details TTL"
    );
}

#[test]
fn movie_no_info_falls_back_to_details_ttl() {
    let cfg = CacheTtlConfig::default();
    let ttl = cfg.movie_details_ttl(&movie(None, None));
    assert_eq!(ttl, cfg.details);
}

#[test]
fn tv_active_in_production_gets_short_ttl() {
    let cfg = CacheTtlConfig::default();
    let ttl = cfg.tv_show_details_ttl(&tv(
        Some(days_ago(1000)),
        None,
        Some("Returning Series"),
        Some(true),
    ));
    assert_eq!(ttl, cfg.active_content_details_ttl);
    assert!(
        ttl <= Duration::from_secs(4 * 3600),
        "active TV show must be cached for ≤4h"
    );
}

#[test]
fn tv_recently_aired_gets_short_ttl() {
    let cfg = CacheTtlConfig::default();
    let last_air = days_ago(30); // aired 30 days ago
    let ttl = cfg.tv_show_details_ttl(&tv(
        Some(days_ago(2000)),
        Some(last_air),
        Some("Returning Series"),
        Some(false),
    ));
    assert_eq!(ttl, cfg.active_content_details_ttl);
    assert!(ttl <= Duration::from_secs(4 * 3600));
}

#[test]
fn tv_old_ended_gets_long_ttl() {
    let cfg = CacheTtlConfig::default();
    let first_air = days_ago(2000); // ~5.5 years ago
    let last_air = days_ago(1800); // ended ~5 years ago
    let ttl = cfg.tv_show_details_ttl(&tv(
        Some(first_air),
        Some(last_air),
        Some("Ended"),
        Some(false),
    ));
    assert_eq!(
        ttl, cfg.old_content_details_ttl,
        "old ended show should use old_content_details_ttl"
    );
}

#[test]
fn tv_canceled_old_gets_long_ttl() {
    let cfg = CacheTtlConfig::default();
    let first_air = days_ago(1500);
    let ttl = cfg.tv_show_details_ttl(&tv(Some(first_air), None, Some("Canceled"), Some(false)));
    assert_eq!(ttl, cfg.old_content_details_ttl);
}

#[test]
fn tv_returning_series_gets_medium_ttl() {
    let cfg = CacheTtlConfig::default();
    // Show started 3 years ago, returning series but hasn't aired in >90 days
    let first_air = days_ago(1100);
    let last_air = days_ago(200);
    let ttl = cfg.tv_show_details_ttl(&tv(
        Some(first_air),
        Some(last_air),
        Some("Returning Series"),
        Some(false),
    ));
    assert_eq!(ttl, cfg.recent_content_details_ttl);
}

#[test]
fn tv_recent_first_air_gets_medium_ttl() {
    let cfg = CacheTtlConfig::default();
    let first_air = days_ago(100); // started 100 days ago
    let last_air = days_ago(95); // last aired 95 days ago (just outside active window)
    let ttl = cfg.tv_show_details_ttl(&tv(
        Some(first_air),
        Some(last_air),
        Some("Ended"),
        Some(false),
    ));
    assert_eq!(ttl, cfg.recent_content_details_ttl);
}

#[test]
fn tv_no_info_falls_back_to_details_ttl() {
    let cfg = CacheTtlConfig::default();
    let ttl = cfg.tv_show_details_ttl(&tv(None, None, None, None));
    assert_eq!(ttl, cfg.details);
}

#[test]
fn tv_active_overrides_old_status() {
    let cfg = CacheTtlConfig::default();
    // Even though show is old, if in_production=true it should use active TTL
    let first_air = days_ago(2000);
    let ttl = cfg.tv_show_details_ttl(&tv(Some(first_air), None, Some("Ended"), Some(true)));
    assert_eq!(
        ttl, cfg.active_content_details_ttl,
        "in_production=true overrides old/ended status"
    );
}

// ── Partial dates and status fallback ──────────────────────────────────────────

fn this_year() -> i32 {
    use chrono::Datelike;
    chrono::Utc::now().date_naive().year()
}

#[test]
fn movie_year_only_old_release_gets_long_ttl() {
    let cfg = CacheTtlConfig::default();
    // A year-only date ten years back counts as January 1st of that year.
    let release = PartialDate::from_year(this_year() - 10);
    let ttl = cfg.movie_details_ttl(&movie(Some(release), Some("FINISHED")));
    assert_eq!(ttl, cfg.old_content_details_ttl);
}

#[test]
fn movie_year_month_counts_as_first_of_month() {
    // Threshold sits between the 1st and the last day of a past month, so the
    // result depends on which day of the month the date is taken as.
    let first = PartialDate::from(chrono::Utc::now().date_naive() - chrono::Duration::days(400));
    let month = first.month().expect("full date has a month");
    let start = chrono::NaiveDate::from_ymd_opt(first.year(), u32::from(month), 1)
        .expect("valid first of month");
    let days_to_start = (chrono::Utc::now().date_naive() - start).num_days();
    let threshold = u32::try_from(days_to_start - 1).expect("positive threshold");
    let cfg = CacheTtlConfig::default().with_old_content_threshold_days(threshold);
    let year_month = PartialDate::year_month(first.year(), month).expect("valid month");
    let ttl = cfg.movie_details_ttl(&movie(Some(year_month), Some("Released")));
    assert_eq!(ttl, cfg.old_content_details_ttl);
}

#[test]
fn movie_impossible_day_counts_as_first_of_month() {
    let cfg = CacheTtlConfig::default();
    let release = PartialDate::ymd(this_year() - 10, 2, 30).expect("in-range components");
    let ttl = cfg.movie_details_ttl(&movie(Some(release), Some("Released")));
    assert_eq!(ttl, cfg.old_content_details_ttl);
}

#[test]
fn movie_status_falls_back_to_base_movie_status() {
    let cfg = CacheTtlConfig::default();
    let base = UnifiedMovie::new(MediaId::tmdb(550), "Fight Club")
        .with_release_date(days_ago(2000))
        .with_status("Released");
    let ttl = cfg.movie_details_ttl(&UnifiedMovieDetails::new(base));
    assert_eq!(ttl, cfg.old_content_details_ttl);
}

#[test]
fn tv_year_only_old_ended_gets_long_ttl() {
    let cfg = CacheTtlConfig::default();
    let first_air = PartialDate::from_year(this_year() - 10);
    let ttl = cfg.tv_show_details_ttl(&tv(Some(first_air), None, Some("FINISHED"), None));
    assert_eq!(ttl, cfg.old_content_details_ttl);
}

#[test]
fn tv_status_falls_back_to_base_show_status() {
    let cfg = CacheTtlConfig::default();
    let base = UnifiedTvShow::new(MediaId::tmdb(1399), "Game of Thrones")
        .with_first_air_date(days_ago(2000))
        .with_status("Ended");
    let ttl = cfg.tv_show_details_ttl(&UnifiedTvShowDetails::new(base));
    assert_eq!(ttl, cfg.old_content_details_ttl);
}

#[test]
fn tv_unknown_in_production_falls_through_to_date_rules() {
    let cfg = CacheTtlConfig::default();
    let ttl = cfg.tv_show_details_ttl(&tv(Some(days_ago(2000)), None, Some("Ended"), None));
    assert_eq!(ttl, cfg.old_content_details_ttl);
}

// ── with_* setters ─────────────────────────────────────────────────────────────

#[test]
fn ttl_setters_override_each_field() {
    let secs = Duration::from_secs;
    let cfg = CacheTtlConfig::default()
        .with_details(secs(1))
        .with_search(secs(2))
        .with_discovery(secs(3))
        .with_items(secs(4))
        .with_old_content_threshold_days(5)
        .with_recent_content_threshold_days(6)
        .with_old_content_details_ttl(secs(7))
        .with_recent_content_details_ttl(secs(8))
        .with_active_content_details_ttl(secs(9));
    assert_eq!(cfg.details, secs(1));
    assert_eq!(cfg.search, secs(2));
    assert_eq!(cfg.discovery, secs(3));
    assert_eq!(cfg.items, secs(4));
    assert_eq!(cfg.old_content_threshold_days, 5);
    assert_eq!(cfg.recent_content_threshold_days, 6);
    assert_eq!(cfg.old_content_details_ttl, secs(7));
    assert_eq!(cfg.recent_content_details_ttl, secs(8));
    assert_eq!(cfg.active_content_details_ttl, secs(9));
}

#[test]
fn ttl_setters_leave_other_fields_at_default() {
    let default = CacheTtlConfig::default();
    let cfg = CacheTtlConfig::default().with_search(Duration::from_secs(1));
    assert_eq!(cfg.details, default.details);
    assert_eq!(cfg.discovery, default.discovery);
    assert_eq!(cfg.items, default.items);
    assert_eq!(
        cfg.old_content_threshold_days,
        default.old_content_threshold_days
    );
}
