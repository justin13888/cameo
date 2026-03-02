#![cfg(feature = "cache")]

use std::time::Duration;

use cameo::cache::{CacheBackend, CacheKey, CacheTtlConfig, MediaType, SqliteCache};

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
            media_type: None,
            query: "q".to_string(),
            page: 1
        }
        .key_type(),
        "search"
    );
    assert_eq!(
        CacheKey::Discovery {
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
        media_type: Some(MediaType::Movie),
        query: "  Dune  ".to_string(),
        page: 1,
    };
    let k2 = CacheKey::Search {
        media_type: Some(MediaType::Movie),
        query: "dune".to_string(),
        page: 1,
    };
    assert_eq!(k1.key_id(), k2.key_id());
}

// ── Age-aware TTL policy ───────────────────────────────────────────────────────

fn days_ago(n: i64) -> String {
    use chrono::{Duration, Utc};
    (Utc::now().date_naive() - Duration::days(n))
        .format("%Y-%m-%d")
        .to_string()
}

#[test]
fn movie_old_released_gets_long_ttl() {
    let cfg = CacheTtlConfig::default();
    let release = days_ago(1200); // ~3.3 years ago
    let ttl = cfg.movie_details_ttl(Some(&release), Some("Released"));
    assert_eq!(
        ttl, cfg.old_content_details_ttl,
        "old released movie should use old_content_details_ttl"
    );
}

#[test]
fn movie_recent_gets_medium_ttl() {
    let cfg = CacheTtlConfig::default();
    let release = days_ago(60); // 2 months ago
    let ttl = cfg.movie_details_ttl(Some(&release), Some("Released"));
    assert_eq!(
        ttl, cfg.recent_content_details_ttl,
        "recently released movie should use recent_content_details_ttl"
    );
}

#[test]
fn movie_in_production_gets_medium_ttl() {
    let cfg = CacheTtlConfig::default();
    let ttl = cfg.movie_details_ttl(None, Some("In Production"));
    assert_eq!(
        ttl, cfg.recent_content_details_ttl,
        "in-production movie should use recent_content_details_ttl"
    );
}

#[test]
fn movie_post_production_gets_medium_ttl() {
    let cfg = CacheTtlConfig::default();
    let ttl = cfg.movie_details_ttl(Some(&days_ago(30)), Some("Post Production"));
    assert_eq!(ttl, cfg.recent_content_details_ttl);
}

#[test]
fn movie_released_recently_gets_medium_even_if_status_missing() {
    let cfg = CacheTtlConfig::default();
    let release = days_ago(90);
    let ttl = cfg.movie_details_ttl(Some(&release), None);
    assert_eq!(ttl, cfg.recent_content_details_ttl);
}

#[test]
fn movie_released_mid_range_falls_back_to_details_ttl() {
    let cfg = CacheTtlConfig::default();
    // 500 days ago: not "old" (>1095) and not "recent" (<180), status "Released"
    let release = days_ago(500);
    let ttl = cfg.movie_details_ttl(Some(&release), Some("Released"));
    assert_eq!(
        ttl, cfg.details,
        "mid-range released movie should fall back to details TTL"
    );
}

#[test]
fn movie_no_info_falls_back_to_details_ttl() {
    let cfg = CacheTtlConfig::default();
    let ttl = cfg.movie_details_ttl(None, None);
    assert_eq!(ttl, cfg.details);
}

#[test]
fn tv_active_in_production_gets_short_ttl() {
    let cfg = CacheTtlConfig::default();
    let ttl = cfg.tv_show_details_ttl(Some(&days_ago(1000)), None, Some("Returning Series"), true);
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
    let ttl = cfg.tv_show_details_ttl(
        Some(&days_ago(2000)),
        Some(&last_air),
        Some("Returning Series"),
        false,
    );
    assert_eq!(ttl, cfg.active_content_details_ttl);
    assert!(ttl <= Duration::from_secs(4 * 3600));
}

#[test]
fn tv_old_ended_gets_long_ttl() {
    let cfg = CacheTtlConfig::default();
    let first_air = days_ago(2000); // ~5.5 years ago
    let last_air = days_ago(1800); // ended ~5 years ago
    let ttl = cfg.tv_show_details_ttl(Some(&first_air), Some(&last_air), Some("Ended"), false);
    assert_eq!(
        ttl, cfg.old_content_details_ttl,
        "old ended show should use old_content_details_ttl"
    );
}

#[test]
fn tv_canceled_old_gets_long_ttl() {
    let cfg = CacheTtlConfig::default();
    let first_air = days_ago(1500);
    let ttl = cfg.tv_show_details_ttl(Some(&first_air), None, Some("Canceled"), false);
    assert_eq!(ttl, cfg.old_content_details_ttl);
}

#[test]
fn tv_returning_series_gets_medium_ttl() {
    let cfg = CacheTtlConfig::default();
    // Show started 3 years ago, returning series but hasn't aired in >90 days
    let first_air = days_ago(1100);
    let last_air = days_ago(200);
    let ttl = cfg.tv_show_details_ttl(
        Some(&first_air),
        Some(&last_air),
        Some("Returning Series"),
        false,
    );
    assert_eq!(ttl, cfg.recent_content_details_ttl);
}

#[test]
fn tv_recent_first_air_gets_medium_ttl() {
    let cfg = CacheTtlConfig::default();
    let first_air = days_ago(100); // started 100 days ago
    let last_air = days_ago(95); // last aired 95 days ago (just outside active window)
    let ttl = cfg.tv_show_details_ttl(Some(&first_air), Some(&last_air), Some("Ended"), false);
    assert_eq!(ttl, cfg.recent_content_details_ttl);
}

#[test]
fn tv_no_info_falls_back_to_details_ttl() {
    let cfg = CacheTtlConfig::default();
    let ttl = cfg.tv_show_details_ttl(None, None, None, false);
    assert_eq!(ttl, cfg.details);
}

#[test]
fn tv_active_overrides_old_status() {
    let cfg = CacheTtlConfig::default();
    // Even though show is old, if in_production=true it should use active TTL
    let first_air = days_ago(2000);
    let ttl = cfg.tv_show_details_ttl(Some(&first_air), None, Some("Ended"), true);
    assert_eq!(
        ttl, cfg.active_content_details_ttl,
        "in_production=true overrides old/ended status"
    );
}
