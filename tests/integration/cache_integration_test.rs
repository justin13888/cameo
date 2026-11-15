#![cfg(all(feature = "cache", feature = "tmdb"))]

use std::sync::Arc;

use cameo::{MediaId, cache::SqliteCache, providers::tmdb::TmdbConfig, unified::CameoClient};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path, query_param},
};

fn search_movie_fixture() -> &'static str {
    include_str!("../fixtures/search_movie_response.json")
}

fn movie_details_fixture() -> &'static str {
    include_str!("../fixtures/movie_details_response.json")
}

async fn setup(server: &MockServer) -> CameoClient {
    let backend = Arc::new(SqliteCache::in_memory().unwrap());
    let config = TmdbConfig::new_with_base_url("test-token", server.uri());
    CameoClient::builder()
        .with_tmdb(config)
        .with_cache_backend(backend)
        .build()
        .unwrap()
}

// ── Search caching ────────────────────────────────────────────────────────────

#[tokio::test]
async fn search_result_is_cached_second_call_skips_server() {
    let server = MockServer::start().await;
    let client = setup(&server).await;

    Mock::given(method("GET"))
        .and(path("/3/search/movie"))
        .and(query_param("query", "fight club"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(search_movie_fixture(), "application/json"),
        )
        .expect(1) // must only be called ONCE
        .mount(&server)
        .await;

    let r1 = client.search_movies("fight club", None).await.unwrap();
    let r2 = client.search_movies("fight club", None).await.unwrap();

    assert_eq!(r1.results.len(), 1);
    assert_eq!(r2.results.len(), 1);
    assert_eq!(r1.results[0].provider_id, r2.results[0].provider_id);
    // Wiremock will panic at drop if the mock was called more than once.
}

#[tokio::test]
async fn search_indexes_individual_items_into_item_cache() {
    let server = MockServer::start().await;
    let client = setup(&server).await;

    Mock::given(method("GET"))
        .and(path("/3/search/movie"))
        .and(query_param("query", "fight club"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(search_movie_fixture(), "application/json"),
        )
        .mount(&server)
        .await;

    // Trigger the search so the result is cached and items are indexed.
    client.search_movies("fight club", None).await.unwrap();

    // The item for "tmdb:550" should be retrievable from cache without another HTTP call.
    let cached = client.cached_movie(&MediaId::tmdb(550)).await;
    assert!(cached.is_some(), "item should be indexed after search");
    assert_eq!(cached.unwrap().provider_id, "tmdb:550");
}

// ── Detail caching ────────────────────────────────────────────────────────────

#[tokio::test]
async fn detail_result_is_cached_second_call_skips_server() {
    let server = MockServer::start().await;
    let client = setup(&server).await;

    Mock::given(method("GET"))
        .and(path("/3/movie/550"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(movie_details_fixture(), "application/json"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let d1 = client.movie_details(&MediaId::tmdb(550)).await.unwrap();
    let d2 = client.movie_details(&MediaId::tmdb(550)).await.unwrap();

    assert_eq!(d1.movie.provider_id, d2.movie.provider_id);
}

#[tokio::test]
async fn detail_fetch_populates_both_detail_and_item_caches() {
    let server = MockServer::start().await;
    let client = setup(&server).await;

    Mock::given(method("GET"))
        .and(path("/3/movie/550"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(movie_details_fixture(), "application/json"),
        )
        .mount(&server)
        .await;

    let fetched = client.movie_details(&MediaId::tmdb(550)).await.unwrap();

    // Detail cache should be populated.
    let detail = client.cached_movie_details(&MediaId::tmdb(550)).await;
    assert!(detail.is_some(), "detail cache should be populated");
    assert_eq!(
        detail.as_ref(),
        Some(&fetched),
        "cached details equal the fetched details"
    );

    // Item cache should also be populated.
    let item = client.cached_movie(&MediaId::tmdb(550)).await;
    assert!(
        item.is_some(),
        "item cache should be populated after detail fetch"
    );
    assert_eq!(
        item,
        Some(fetched.movie),
        "cached item equals the fetched movie"
    );
}

/// What a cache hit returns equals the uncached result field for field, for
/// both the search page and the item it indexed.
#[tokio::test]
async fn cached_search_results_equal_the_uncached_result() {
    let server = MockServer::start().await;
    let client = setup(&server).await;

    Mock::given(method("GET"))
        .and(path("/3/search/movie"))
        .and(query_param("query", "fight club"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(search_movie_fixture(), "application/json"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let uncached = client.search_movies("fight club", None).await.unwrap();
    let cached = client.search_movies("fight club", None).await.unwrap();
    assert_eq!(cached, uncached);
    assert_eq!(
        client.cached_movie(&MediaId::tmdb(550)).await,
        Some(uncached.results[0].clone())
    );
}

// ── Cache miss cases ──────────────────────────────────────────────────────────

#[tokio::test]
async fn cached_movie_details_returns_none_if_only_search_was_done() {
    let server = MockServer::start().await;
    let client = setup(&server).await;

    Mock::given(method("GET"))
        .and(path("/3/search/movie"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(search_movie_fixture(), "application/json"),
        )
        .mount(&server)
        .await;

    // Do a search (no detail fetch).
    client.search_movies("fight club", None).await.unwrap();

    // Item should be in cache, but NOT the full details.
    let detail = client.cached_movie_details(&MediaId::tmdb(550)).await;
    assert!(
        detail.is_none(),
        "details should not be available after search-only"
    );
}

// ── Invalidation ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn invalidate_cached_removes_all_entries_for_provider_id() {
    let server = MockServer::start().await;
    let client = setup(&server).await;

    Mock::given(method("GET"))
        .and(path("/3/movie/550"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(movie_details_fixture(), "application/json"),
        )
        .mount(&server)
        .await;

    client.movie_details(&MediaId::tmdb(550)).await.unwrap();

    // Verify entries exist.
    assert!(client.cached_movie(&MediaId::tmdb(550)).await.is_some());
    assert!(
        client
            .cached_movie_details(&MediaId::tmdb(550))
            .await
            .is_some()
    );

    client.invalidate_cached(&MediaId::tmdb(550)).await;

    assert!(client.cached_movie(&MediaId::tmdb(550)).await.is_none());
    assert!(
        client
            .cached_movie_details(&MediaId::tmdb(550))
            .await
            .is_none()
    );
}

#[tokio::test]
async fn clear_cache_empties_everything() {
    let server = MockServer::start().await;
    let client = setup(&server).await;

    Mock::given(method("GET"))
        .and(path("/3/movie/550"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(movie_details_fixture(), "application/json"),
        )
        .mount(&server)
        .await;

    client.movie_details(&MediaId::tmdb(550)).await.unwrap();
    assert!(client.cached_movie(&MediaId::tmdb(550)).await.is_some());

    client.clear_cache().await;

    assert!(client.cached_movie(&MediaId::tmdb(550)).await.is_none());
    assert!(
        client
            .cached_movie_details(&MediaId::tmdb(550))
            .await
            .is_none()
    );
}

// ── Age-aware detail TTLs ────────────────────────────────────────────────────

mod age_aware_ttl {
    use std::{
        sync::{Arc, Mutex},
        time::Duration,
    };

    use async_trait::async_trait;
    use cameo::{
        MediaId,
        cache::{CacheBackend, CacheError, CacheKey, CacheTtlConfig, MediaType},
        providers::tmdb::TmdbConfig,
        unified::CameoClient,
    };
    use chrono::{Days, Utc};
    use serde_json::Value;
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{method, path},
    };

    const DETAILS: Duration = Duration::from_secs(11);
    const OLD: Duration = Duration::from_secs(22);
    const RECENT: Duration = Duration::from_secs(33);
    const ACTIVE: Duration = Duration::from_secs(44);

    /// A backend that stores nothing and records the TTL of every write.
    #[derive(Default)]
    struct Recording {
        writes: Mutex<Vec<(CacheKey, Duration)>>,
    }

    impl Recording {
        /// The TTL the detail entry for `provider_id` was written with.
        fn detail_ttl(&self, media_type: MediaType, provider_id: &str) -> Option<Duration> {
            let key = CacheKey::Detail {
                media_type,
                provider_id: provider_id.to_string(),
            };
            self.writes
                .lock()
                .expect("writes")
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, ttl)| *ttl)
        }
    }

    #[async_trait]
    impl CacheBackend for Recording {
        async fn get(&self, _key: &CacheKey) -> Result<Option<Value>, CacheError> {
            Ok(None)
        }
        async fn set(&self, key: CacheKey, _value: Value, ttl: Duration) -> Result<(), CacheError> {
            self.writes.lock().expect("writes").push((key, ttl));
            Ok(())
        }
        async fn invalidate(&self, _key: &CacheKey) -> Result<(), CacheError> {
            Ok(())
        }
        async fn clear(&self) -> Result<(), CacheError> {
            Ok(())
        }
    }

    fn ttls() -> CacheTtlConfig {
        let mut ttl = CacheTtlConfig::default();
        ttl.details = DETAILS;
        ttl.old_content_details_ttl = OLD;
        ttl.recent_content_details_ttl = RECENT;
        ttl.active_content_details_ttl = ACTIVE;
        ttl
    }

    /// `days` days before today, as `YYYY-MM-DD`.
    fn days_ago(days: u64) -> String {
        Utc::now()
            .date_naive()
            .checked_sub_days(Days::new(days))
            .expect("in range")
            .format("%Y-%m-%d")
            .to_string()
    }

    /// A fixture with each `(field, value)` overwritten.
    fn fixture(raw: &str, overrides: &[(&str, Value)]) -> Value {
        let mut body: Value = serde_json::from_str(raw).expect("fixture is JSON");
        for (field, value) in overrides {
            body[*field] = value.clone();
        }
        body
    }

    /// Serve `body` at `endpoint` and return a client caching into `backend`.
    async fn serve(
        server: &MockServer,
        endpoint: &str,
        body: Value,
    ) -> (CameoClient, Arc<Recording>) {
        Mock::given(method("GET"))
            .and(path(endpoint))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .mount(server)
            .await;
        let backend = Arc::new(Recording::default());
        let client = CameoClient::builder()
            .with_tmdb(TmdbConfig::new_with_base_url("test-token", server.uri()))
            .with_cache_backend(backend.clone())
            .with_cache_ttl(ttls())
            .build()
            .expect("configured");
        (client, backend)
    }

    async fn movie_ttl(overrides: &[(&str, Value)]) -> Option<Duration> {
        let server = MockServer::start().await;
        let body = fixture(super::movie_details_fixture(), overrides);
        let (client, backend) = serve(&server, "/3/movie/550", body).await;
        client
            .movie_details(&MediaId::tmdb(550))
            .await
            .expect("details");
        backend.detail_ttl(MediaType::Movie, "tmdb:550")
    }

    async fn tv_ttl(overrides: &[(&str, Value)]) -> Option<Duration> {
        let server = MockServer::start().await;
        let raw = include_str!("../fixtures/tv_details_response.json");
        let body = fixture(raw, overrides);
        let (client, backend) = serve(&server, "/3/tv/1396", body).await;
        client
            .tv_show_details(&MediaId::tmdb(1396))
            .await
            .expect("details");
        backend.detail_ttl(MediaType::TvShow, "tmdb:1396")
    }

    #[tokio::test]
    async fn an_old_released_movie_gets_the_old_content_ttl() {
        let ttl = movie_ttl(&[("release_date", "1999-10-15".into())]).await;
        assert_eq!(ttl, Some(OLD));
    }

    /// A year-only date counts as the first day of that year.
    #[tokio::test]
    async fn a_year_only_old_release_gets_the_old_content_ttl() {
        let ttl = movie_ttl(&[("release_date", "1999".into())]).await;
        assert_eq!(ttl, Some(OLD));
    }

    #[tokio::test]
    async fn a_recent_movie_gets_the_recent_content_ttl() {
        let ttl = movie_ttl(&[("release_date", days_ago(10).into())]).await;
        assert_eq!(ttl, Some(RECENT));
    }

    #[tokio::test]
    async fn a_movie_no_age_rule_matches_gets_the_details_ttl() {
        let ttl = movie_ttl(&[("release_date", days_ago(2 * 365).into())]).await;
        assert_eq!(ttl, Some(DETAILS));
    }

    #[tokio::test]
    async fn an_old_ended_show_gets_the_old_content_ttl() {
        let ttl = tv_ttl(&[
            ("first_air_date", "2008-01-20".into()),
            ("last_air_date", "2013-09-29".into()),
            ("status", "Ended".into()),
            ("in_production", false.into()),
        ])
        .await;
        assert_eq!(ttl, Some(OLD));
    }

    #[tokio::test]
    async fn a_show_in_production_gets_the_active_content_ttl() {
        let ttl = tv_ttl(&[("in_production", true.into())]).await;
        assert_eq!(ttl, Some(ACTIVE));
    }

    #[tokio::test]
    async fn a_recent_returning_show_gets_the_recent_content_ttl() {
        let ttl = tv_ttl(&[
            ("first_air_date", days_ago(30).into()),
            ("last_air_date", days_ago(120).into()),
            ("status", "Returning Series".into()),
            ("in_production", false.into()),
        ])
        .await;
        assert_eq!(ttl, Some(RECENT));
    }
}
