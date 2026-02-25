#![cfg(feature = "cache")]

use std::time::Duration;

use cameo::cache::{CacheBackend, CacheKey, MediaType, SqliteCache};

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

    cache.set(key.clone(), value.clone(), Duration::from_secs(60)).await.unwrap();
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
        .set(key.clone(), serde_json::json!({ "v": 1 }), Duration::from_secs(60))
        .await
        .unwrap();
    cache
        .set(key.clone(), serde_json::json!({ "v": 2 }), Duration::from_secs(60))
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
        .set(key.clone(), serde_json::json!([1, 2, 3]), Duration::from_secs(60))
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
    cache.set(k1.clone(), serde_json::json!(1), Duration::from_secs(60)).await.unwrap();
    cache.set(k2.clone(), serde_json::json!(2), Duration::from_secs(60)).await.unwrap();

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
        .set(key.clone(), serde_json::json!({ "expired": true }), Duration::from_secs(1))
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
        .set(detail_key.clone(), serde_json::json!({ "type": "detail" }), Duration::from_secs(60))
        .await
        .unwrap();
    cache
        .set(item_key.clone(), serde_json::json!({ "type": "item" }), Duration::from_secs(60))
        .await
        .unwrap();

    assert_eq!(cache.get(&detail_key).await.unwrap().unwrap()["type"], "detail");
    assert_eq!(cache.get(&item_key).await.unwrap().unwrap()["type"], "item");
}

// ── Key serialization ──

#[test]
fn cache_key_type_discriminants() {
    assert_eq!(
        CacheKey::Detail { media_type: MediaType::Movie, provider_id: "tmdb:550".to_string() }
            .key_type(),
        "detail"
    );
    assert_eq!(
        CacheKey::Item { media_type: MediaType::Movie, provider_id: "tmdb:550".to_string() }
            .key_type(),
        "item"
    );
    assert_eq!(
        CacheKey::Search { media_type: None, query: "q".to_string(), page: 1 }.key_type(),
        "search"
    );
    assert_eq!(
        CacheKey::Discovery { endpoint: "e".to_string(), page: 1 }.key_type(),
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
