#![cfg(all(feature = "cache", feature = "tmdb"))]

use std::sync::Arc;

use cameo::{
    cache::SqliteCache,
    providers::tmdb::TmdbConfig,
    unified::{CameoClient, DetailProvider, SearchProvider},
};
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
    client.flush_cache_writes().await;
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
    client.flush_cache_writes().await;

    // The item for "tmdb:550" should be retrievable from cache without another HTTP call.
    let cached = client.cached_movie("tmdb:550").await;
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

    let d1 = client.movie_details(550).await.unwrap();
    client.flush_cache_writes().await;
    let d2 = client.movie_details(550).await.unwrap();

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

    client.movie_details(550).await.unwrap();
    client.flush_cache_writes().await;

    // Detail cache should be populated.
    let detail = client.cached_movie_details("tmdb:550").await;
    assert!(detail.is_some(), "detail cache should be populated");

    // Item cache should also be populated.
    let item = client.cached_movie("tmdb:550").await;
    assert!(
        item.is_some(),
        "item cache should be populated after detail fetch"
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
    client.flush_cache_writes().await;

    // Item should be in cache, but NOT the full details.
    let detail = client.cached_movie_details("tmdb:550").await;
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

    client.movie_details(550).await.unwrap();
    client.flush_cache_writes().await;

    // Verify entries exist.
    assert!(client.cached_movie("tmdb:550").await.is_some());
    assert!(client.cached_movie_details("tmdb:550").await.is_some());

    client.invalidate_cached("tmdb:550").await;

    assert!(client.cached_movie("tmdb:550").await.is_none());
    assert!(client.cached_movie_details("tmdb:550").await.is_none());
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

    client.movie_details(550).await.unwrap();
    client.flush_cache_writes().await;
    assert!(client.cached_movie("tmdb:550").await.is_some());

    client.clear_cache().await;

    assert!(client.cached_movie("tmdb:550").await.is_none());
    assert!(client.cached_movie_details("tmdb:550").await.is_none());
}
