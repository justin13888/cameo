use std::time::Duration;

use cameo::{
    RateLimit, into_stream,
    providers::tmdb::{TmdbClient, TmdbConfig},
};
use futures::StreamExt;
use serde_json::json;
use wiremock::{
    Mock, MockServer, Request, Respond, ResponseTemplate,
    matchers::{method, path, query_param},
};

fn setup_tmdb_client(server: &MockServer) -> TmdbClient {
    TmdbClient::new(TmdbConfig::new_with_base_url("test-token", server.uri())).unwrap()
}

/// Bound on a whole-stream test, so a stream that never ends fails instead of
/// hanging the suite.
const STREAM_BOUND: Duration = Duration::from_secs(60);

/// A list response for the requested `page` (default `1`) holding one movie
/// with id `page`, claiming `total_pages` pages: more than TMDB lets a client
/// request.
struct EchoPage {
    total_pages: u32,
}

impl Respond for EchoPage {
    fn respond(&self, request: &Request) -> ResponseTemplate {
        let page: u32 = request
            .url
            .query_pairs()
            .find(|(k, _)| k == "page")
            .and_then(|(_, v)| v.parse().ok())
            .unwrap_or(1);
        ResponseTemplate::new(200).set_body_json(json!({
            "page": page,
            "results": [{ "id": page, "title": format!("Movie {page}") }],
            "total_pages": self.total_pages,
            "total_results": u64::from(self.total_pages) * 20,
        }))
    }
}

// ── Trending ──────────────────────────────────────────────────────────────────

#[tokio::test]
async fn trending_movies_deserializes_response() {
    let server = MockServer::start().await;
    let client = setup_tmdb_client(&server);

    Mock::given(method("GET"))
        .and(path("/3/trending/movie/week"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            include_str!("../fixtures/trending_movies_response.json"),
            "application/json",
        ))
        .mount(&server)
        .await;

    let page = client
        .trending_movies(cameo::TimeWindow::Week)
        .await
        .unwrap();

    assert_eq!(page.page, 1);
    assert_eq!(page.total_results, 1);
    assert_eq!(page.results.len(), 1);
    assert_eq!(page.results[0].provider_id, "tmdb:550");
    assert_eq!(page.results[0].title, "Fight Club");
}

/// Trending ignores the page argument, so a body claiming more pages still
/// yields a single page, and a stream over it ends after one request.
#[tokio::test]
async fn trending_movies_is_a_single_page_and_its_stream_terminates() {
    let server = MockServer::start().await;
    let client = setup_tmdb_client(&server);

    Mock::given(method("GET"))
        .and(path("/3/trending/movie/day"))
        .respond_with(EchoPage { total_pages: 1000 })
        .expect(2)
        .mount(&server)
        .await;

    let page = client
        .trending_movies(cameo::TimeWindow::Day)
        .await
        .unwrap();
    assert!(!page.has_next);
    assert_eq!(page.next, None);
    assert_eq!(page.total_pages, 1);
    assert_eq!(page.total_results, 20_000);

    let stream = into_stream(move |_page| {
        let client = client.clone();
        async move { client.trending_movies(cameo::TimeWindow::Day).await }
    });
    let items: Vec<_> = tokio::time::timeout(STREAM_BOUND, stream.collect::<Vec<_>>())
        .await
        .expect("trending stream terminates");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].as_ref().unwrap().provider_id, "tmdb:1");
}

// ── Popular ───────────────────────────────────────────────────────────────────

/// Page 500 of a result set TMDB says is longer is the last page the client
/// can request, so it advertises no next page and caps `total_pages` at 500.
#[tokio::test]
async fn popular_movies_page_500_is_the_last_page() {
    let server = MockServer::start().await;
    let client = setup_tmdb_client(&server);

    Mock::given(method("GET"))
        .and(path("/3/movie/popular"))
        .and(query_param("page", "500"))
        .respond_with(EchoPage { total_pages: 1000 })
        .mount(&server)
        .await;

    let page = client.popular_movies(Some(500)).await.unwrap();

    assert_eq!(page.page, 500);
    assert!(!page.has_next);
    assert_eq!(page.next, None);
    assert_eq!(page.total_pages, 500);
    assert_eq!(page.total_results, 20_000);
}

/// A stream over a list TMDB says has 1000 pages ends after page 500 instead
/// of re-fetching page 500 forever.
#[tokio::test]
async fn popular_movies_stream_terminates_at_the_page_limit() {
    let server = MockServer::start().await;
    let client = TmdbClient::new(
        TmdbConfig::new_with_base_url("test-token", server.uri())
            .with_rate_limit(RateLimit::per(10_000, Duration::from_secs(1))),
    )
    .unwrap();

    Mock::given(method("GET"))
        .and(path("/3/movie/popular"))
        .respond_with(EchoPage { total_pages: 1000 })
        .expect(500)
        .mount(&server)
        .await;

    let stream = into_stream(move |page| {
        let client = client.clone();
        async move { client.popular_movies(Some(page)).await }
    });
    let items: Vec<_> = tokio::time::timeout(STREAM_BOUND, stream.collect::<Vec<_>>())
        .await
        .expect("popular stream terminates");

    assert_eq!(items.len(), 500);
    assert_eq!(items[499].as_ref().unwrap().provider_id, "tmdb:500");
}

#[tokio::test]
async fn popular_movies_deserializes_response() {
    let server = MockServer::start().await;
    let client = setup_tmdb_client(&server);

    Mock::given(method("GET"))
        .and(path("/3/movie/popular"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            include_str!("../fixtures/popular_movies_response.json"),
            "application/json",
        ))
        .mount(&server)
        .await;

    let page = client.popular_movies(None).await.unwrap();

    assert_eq!(page.page, 1);
    assert_eq!(page.total_results, 200);
    assert_eq!(page.results.len(), 1);
    assert_eq!(page.results[0].provider_id, "tmdb:157336");
    assert_eq!(page.results[0].title, "Interstellar");
}

// ── Top Rated ─────────────────────────────────────────────────────────────────

#[tokio::test]
async fn top_rated_movies_deserializes_response() {
    let server = MockServer::start().await;
    let client = setup_tmdb_client(&server);

    Mock::given(method("GET"))
        .and(path("/3/movie/top_rated"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            include_str!("../fixtures/top_rated_movies_response.json"),
            "application/json",
        ))
        .mount(&server)
        .await;

    let page = client.top_rated_movies(None).await.unwrap();

    assert_eq!(page.page, 1);
    assert_eq!(page.total_results, 100);
    assert_eq!(page.results.len(), 1);
    assert_eq!(page.results[0].provider_id, "tmdb:278");
    assert_eq!(page.results[0].title, "The Shawshank Redemption");
}

// ── Discover ─────────────────────────────────────────────────────────────────

#[tokio::test]
async fn discover_movies_execute_deserializes_response() {
    let server = MockServer::start().await;
    let client = setup_tmdb_client(&server);

    Mock::given(method("GET"))
        .and(path("/3/discover/movie"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            include_str!("../fixtures/discover_movies_response.json"),
            "application/json",
        ))
        .mount(&server)
        .await;

    let page = client.discover_movies().execute().await.unwrap();

    assert_eq!(page.page, 1);
    assert_eq!(page.total_results, 1);
    assert_eq!(page.results.len(), 1);
    assert_eq!(page.results[0].provider_id, "tmdb:550");
    assert_eq!(page.results[0].title, "Fight Club");
}

/// The discover builder's page 500 of a longer result set is the last page.
#[tokio::test]
async fn discover_movies_page_500_is_the_last_page() {
    let server = MockServer::start().await;
    let client = setup_tmdb_client(&server);

    Mock::given(method("GET"))
        .and(path("/3/discover/movie"))
        .and(query_param("page", "500"))
        .respond_with(EchoPage { total_pages: 1000 })
        .mount(&server)
        .await;

    let page = client.discover_movies().page(500).execute().await.unwrap();

    assert_eq!(page.page, 500);
    assert!(!page.has_next);
    assert_eq!(page.next, None);
    assert_eq!(page.total_pages, 500);
}

#[tokio::test]
async fn discover_tv_execute_deserializes_response() {
    let server = MockServer::start().await;
    let client = setup_tmdb_client(&server);

    Mock::given(method("GET"))
        .and(path("/3/discover/tv"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            include_str!("../fixtures/discover_tv_response.json"),
            "application/json",
        ))
        .mount(&server)
        .await;

    let page = client.discover_tv().execute().await.unwrap();

    assert_eq!(page.page, 1);
    assert_eq!(page.total_results, 1);
    assert_eq!(page.results.len(), 1);
    assert_eq!(page.results[0].provider_id, "tmdb:202250");
    assert_eq!(page.results[0].name, "Dirty Linen");
}
