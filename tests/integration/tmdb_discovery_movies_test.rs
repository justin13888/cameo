use cameo::{
    generated::tmdb::{Client as GeneratedClient, types::TrendingMoviesTimeWindow},
    providers::tmdb::{TmdbClient, TmdbConfig},
};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path},
};

async fn setup_generated_client(server: &MockServer) -> GeneratedClient {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::AUTHORIZATION,
        reqwest::header::HeaderValue::from_static("Bearer test-token"),
    );
    let http = reqwest::ClientBuilder::new()
        .default_headers(headers)
        .build()
        .unwrap();
    GeneratedClient::new_with_client(&server.uri(), http)
}

fn setup_tmdb_client(server: &MockServer) -> TmdbClient {
    TmdbClient::new(TmdbConfig::new_with_base_url("test-token", server.uri())).unwrap()
}

// ── Trending ──────────────────────────────────────────────────────────────────

#[tokio::test]
async fn trending_movies_deserializes_response() {
    let server = MockServer::start().await;
    let client = setup_generated_client(&server).await;

    Mock::given(method("GET"))
        .and(path("/3/trending/movie/week"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            include_str!("../fixtures/trending_movies_response.json"),
            "application/json",
        ))
        .mount(&server)
        .await;

    let resp = client
        .trending_movies(TrendingMoviesTimeWindow::Week, None)
        .await
        .unwrap();
    let body = resp.into_inner();

    assert_eq!(body.page, 1);
    assert_eq!(body.total_results, 1);
    assert_eq!(body.results.len(), 1);
    assert_eq!(body.results[0].id, 550);
    assert_eq!(body.results[0].title.as_deref(), Some("Fight Club"));
}

// ── Popular ───────────────────────────────────────────────────────────────────

#[tokio::test]
async fn popular_movies_deserializes_response() {
    let server = MockServer::start().await;
    let client = setup_generated_client(&server).await;

    Mock::given(method("GET"))
        .and(path("/3/movie/popular"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            include_str!("../fixtures/popular_movies_response.json"),
            "application/json",
        ))
        .mount(&server)
        .await;

    let resp = client.movie_popular_list(None, None, None).await.unwrap();
    let body = resp.into_inner();

    assert_eq!(body.page, 1);
    assert_eq!(body.total_results, 200);
    assert_eq!(body.results.len(), 1);
    assert_eq!(body.results[0].id, 157336);
    assert_eq!(body.results[0].title.as_deref(), Some("Interstellar"));
}

// ── Top Rated ─────────────────────────────────────────────────────────────────

#[tokio::test]
async fn top_rated_movies_deserializes_response() {
    let server = MockServer::start().await;
    let client = setup_generated_client(&server).await;

    Mock::given(method("GET"))
        .and(path("/3/movie/top_rated"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            include_str!("../fixtures/top_rated_movies_response.json"),
            "application/json",
        ))
        .mount(&server)
        .await;

    let resp = client.movie_top_rated_list(None, None, None).await.unwrap();
    let body = resp.into_inner();

    assert_eq!(body.page, 1);
    assert_eq!(body.total_results, 100);
    assert_eq!(body.results.len(), 1);
    assert_eq!(body.results[0].id, 278);
    assert_eq!(
        body.results[0].title.as_deref(),
        Some("The Shawshank Redemption")
    );
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

    let resp = client.discover_movies().execute().await.unwrap();

    assert_eq!(resp.page, 1);
    assert_eq!(resp.total_results, 1);
    assert_eq!(resp.results.len(), 1);
    assert_eq!(resp.results[0].id, 550);
    assert_eq!(resp.results[0].title.as_deref(), Some("Fight Club"));
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

    let resp = client.discover_tv().execute().await.unwrap();

    assert_eq!(resp.page, 1);
    assert_eq!(resp.total_results, 1);
    assert_eq!(resp.results.len(), 1);
    assert_eq!(resp.results[0].id, 202250);
    assert_eq!(resp.results[0].name.as_deref(), Some("Dirty Linen"));
}
