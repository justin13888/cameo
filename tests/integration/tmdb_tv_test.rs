use cameo::generated::tmdb::Client as GeneratedClient;
#[cfg(feature = "live-tests")]
use cameo::providers::tmdb::{TmdbClient, TmdbConfig};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path, query_param},
};

fn search_tv_fixture() -> &'static str {
    include_str!("../fixtures/search_tv_response.json")
}

fn tv_details_fixture() -> &'static str {
    include_str!("../fixtures/tv_details_response.json")
}

async fn setup_mock_client() -> (MockServer, GeneratedClient) {
    let server = MockServer::start().await;
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::AUTHORIZATION,
        reqwest::header::HeaderValue::from_static("Bearer test-token"),
    );
    let http = reqwest::ClientBuilder::new()
        .default_headers(headers)
        .build()
        .unwrap();
    let client = GeneratedClient::new_with_client(&server.uri(), http);
    (server, client)
}

#[tokio::test]
async fn search_tv_deserializes_response() {
    let (server, client) = setup_mock_client().await;

    Mock::given(method("GET"))
        .and(path("/3/search/tv"))
        .and(query_param("query", "breaking bad"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(search_tv_fixture(), "application/json"),
        )
        .mount(&server)
        .await;

    let resp = client
        .search_tv(None, None, None, None, "breaking bad", None)
        .await
        .unwrap();
    let body = resp.into_inner();

    assert_eq!(body.page, 1);
    assert_eq!(body.total_results, 1);
    assert_eq!(body.results.len(), 1);

    let show = &body.results[0];
    assert_eq!(show.id, 1396);
    assert_eq!(show.name.as_deref(), Some("Breaking Bad"));
    assert_eq!(show.vote_count, 13306);
}

#[tokio::test]
async fn tv_series_details_deserializes_response() {
    let (server, client) = setup_mock_client().await;

    Mock::given(method("GET"))
        .and(path("/3/tv/1396"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(tv_details_fixture(), "application/json"),
        )
        .mount(&server)
        .await;

    let resp = client.tv_series_details(1396, None, None).await.unwrap();
    let body = resp.into_inner();

    assert_eq!(body.id, 1396);
    assert_eq!(body.name.as_deref(), Some("Breaking Bad"));
    assert_eq!(body.number_of_seasons, 5);
    assert_eq!(body.number_of_episodes, 62);
    assert_eq!(body.status.as_deref(), Some("Ended"));
    assert_eq!(body.genres.len(), 2);
    assert_eq!(body.genres[0].name.as_deref(), Some("Drama"));
}

#[tokio::test]
async fn tv_genres_deserializes_response() {
    let (server, client) = setup_mock_client().await;

    Mock::given(method("GET"))
        .and(path("/3/genre/tv/list"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            include_str!("../fixtures/tv_genres_response.json"),
            "application/json",
        ))
        .mount(&server)
        .await;

    let resp = client.genre_tv_list(None).await.unwrap();
    let body = resp.into_inner();

    assert!(!body.genres.is_empty());
    let drama = body.genres.iter().find(|g| g.id == 18);
    assert!(drama.is_some());
    assert_eq!(drama.unwrap().name.as_deref(), Some("Drama"));
}

#[tokio::test]
async fn trending_tv_deserializes_response() {
    use cameo::generated::tmdb::types::TrendingTvTimeWindow;

    let (server, client) = setup_mock_client().await;

    Mock::given(method("GET"))
        .and(path("/3/trending/tv/week"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            include_str!("../fixtures/trending_tv_response.json"),
            "application/json",
        ))
        .mount(&server)
        .await;

    let resp = client
        .trending_tv(TrendingTvTimeWindow::Week, None)
        .await
        .unwrap();
    let body = resp.into_inner();

    assert_eq!(body.page, 1);
    assert_eq!(body.total_results, 1);
    assert_eq!(body.results.len(), 1);
    assert_eq!(body.results[0].id, 202250);
    assert_eq!(body.results[0].name.as_deref(), Some("Dirty Linen"));
}

#[cfg(feature = "live-tests")]
fn client() -> TmdbClient {
    let token = std::env::var("TMDB_API_TOKEN").expect("TMDB_API_TOKEN must be set for live tests");
    TmdbClient::new(TmdbConfig::new(token).with_language("en-US")).unwrap()
}

#[cfg(feature = "live-tests")]
#[tokio::test]
async fn live_tv_series_details_breaking_bad() {
    let c = client();
    let details = c.tv_series_details(1396).await.unwrap(); // Breaking Bad

    assert_eq!(details.id, 1396);
    assert_eq!(details.name.as_deref(), Some("Breaking Bad"));
    assert!(details.number_of_seasons > 0);
    assert!(!details.genres.is_empty());
}

#[cfg(feature = "live-tests")]
#[tokio::test]
async fn live_trending_tv_returns_page() {
    use cameo::core::config::TimeWindow;
    let c = client();
    let resp = c.trending_tv(TimeWindow::Week, None).await.unwrap();
    assert!(!resp.results.is_empty());
}

#[cfg(feature = "live-tests")]
#[tokio::test]
async fn live_discover_tv_with_filters() {
    use cameo::generated::tmdb::types::DiscoverTvSortBy;

    let c = client();
    let resp = c
        .discover_tv()
        .sort_by(DiscoverTvSortBy::PopularityDesc)
        .with_original_language("en")
        .execute()
        .await
        .unwrap();

    assert!(!resp.results.is_empty());
}
