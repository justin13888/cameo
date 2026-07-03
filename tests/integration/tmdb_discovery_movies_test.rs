use cameo::providers::tmdb::{TmdbClient, TmdbConfig};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path},
};

fn setup_tmdb_client(server: &MockServer) -> TmdbClient {
    TmdbClient::new(TmdbConfig::new_with_base_url("test-token", server.uri())).unwrap()
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
        .trending_movies(cameo::TimeWindow::Week, None)
        .await
        .unwrap();

    assert_eq!(page.page, 1);
    assert_eq!(page.total_results, 1);
    assert_eq!(page.results.len(), 1);
    assert_eq!(page.results[0].provider_id, "tmdb:550");
    assert_eq!(page.results[0].title, "Fight Club");
}

// ── Popular ───────────────────────────────────────────────────────────────────

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
