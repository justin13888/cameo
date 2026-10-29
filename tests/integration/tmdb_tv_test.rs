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

async fn setup_mock_client() -> (MockServer, TmdbClient) {
    let server = MockServer::start().await;
    let client =
        TmdbClient::new(TmdbConfig::new_with_base_url("test-token", server.uri())).unwrap();
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

    let page = client.search_tv_shows("breaking bad", None).await.unwrap();

    assert_eq!(page.page, 1);
    assert_eq!(page.total_results, 1);
    assert_eq!(page.results.len(), 1);

    let show = &page.results[0];
    assert!(show.provider_id == "tmdb:1396");
    assert_eq!(show.name, "Breaking Bad");
    assert_eq!(show.vote_count, Some(13306));
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

    let details = client.tv_series_details(1396).await.unwrap();

    assert!(details.show.provider_id == "tmdb:1396");
    assert_eq!(details.show.name, "Breaking Bad");
    assert_eq!(details.number_of_seasons, Some(5));
    assert_eq!(details.number_of_episodes, Some(62));
    assert_eq!(details.status.as_deref(), Some("Ended"));
    assert_eq!(details.show.genres.len(), 2);
    assert!(details.show.genres.contains(&cameo::Genre::Drama));
    assert_eq!(details.in_production, Some(false));
    assert_eq!(details.show.adult, Some(false));
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

    let genres = client.tv_genres().await.unwrap();

    assert!(!genres.is_empty());
    assert!(genres.contains(&cameo::Genre::Drama));
}

#[tokio::test]
async fn trending_tv_deserializes_response() {
    let (server, client) = setup_mock_client().await;

    Mock::given(method("GET"))
        .and(path("/3/trending/tv/week"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            include_str!("../fixtures/trending_tv_response.json"),
            "application/json",
        ))
        .mount(&server)
        .await;

    let page = client
        .trending_tv(cameo::TimeWindow::Week, None)
        .await
        .unwrap();

    assert_eq!(page.page, 1);
    assert_eq!(page.total_results, 1);
    assert_eq!(page.results.len(), 1);
    assert!(page.results[0].provider_id == "tmdb:202250");
    assert_eq!(page.results[0].name, "Dirty Linen");
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

    assert!(details.show.provider_id == "tmdb:1396");
    assert_eq!(details.show.name, "Breaking Bad");
    assert!(details.number_of_seasons.unwrap_or(0) > 0);
    assert!(!details.show.genres.is_empty());
}

#[cfg(feature = "live-tests")]
#[tokio::test]
async fn live_trending_tv_returns_page() {
    let c = client();
    let resp = c.trending_tv(cameo::TimeWindow::Week, None).await.unwrap();
    assert!(!resp.results.is_empty());
}

#[cfg(feature = "live-tests")]
#[tokio::test]
async fn live_discover_tv_with_filters() {
    use cameo::providers::tmdb::builders::TvSortBy;

    let c = client();
    let resp = c
        .discover_tv()
        .sort_by(TvSortBy::PopularityDesc)
        .with_original_language("en")
        .execute()
        .await
        .unwrap();

    assert!(!resp.results.is_empty());
}

/// Serve `body` as the `/3/tv/1396` response and fetch it through the client.
async fn tv_details_with_body(body: serde_json::Value) -> cameo::UnifiedTvShowDetails {
    let (server, client) = setup_mock_client().await;
    Mock::given(method("GET"))
        .and(path("/3/tv/1396"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&server)
        .await;
    client.tv_series_details(1396).await.unwrap()
}

fn tv_details_json() -> serde_json::Map<String, serde_json::Value> {
    match serde_json::from_str(tv_details_fixture()).unwrap() {
        serde_json::Value::Object(map) => map,
        other => panic!("fixture is not an object: {other:?}"),
    }
}

#[tokio::test]
async fn tv_series_details_accepts_explicit_nulls() {
    // TMDB sometimes sends `null` for fields its spec gives a default; a single
    // null must not fail the whole response.
    let mut body = tv_details_json();
    for key in ["id", "vote_count", "number_of_episodes", "in_production"] {
        body.insert(key.to_string(), serde_json::Value::Null);
    }

    let details = tv_details_with_body(body.into()).await;

    assert_eq!(details.show.name, "Breaking Bad");
    assert_eq!(details.in_production, None);
    // Numeric nulls keep the pre-existing zero fallback.
    assert!(details.show.provider_id == "tmdb:0");
    assert_eq!(details.show.vote_count, Some(0));
    assert_eq!(details.number_of_episodes, Some(0));
    assert_eq!(details.number_of_seasons, Some(5));
}

#[tokio::test]
async fn tv_series_details_missing_in_production_is_none() {
    let mut body = tv_details_json();
    body.remove("in_production");

    let details = tv_details_with_body(body.into()).await;

    // The spec's `default: true` must not turn an absent flag into `true`.
    assert_eq!(details.in_production, None);
}

#[tokio::test]
async fn tv_series_details_in_production_false_is_some_false() {
    let mut body = tv_details_json();
    body.insert("in_production".to_string(), serde_json::Value::Bool(false));

    let details = tv_details_with_body(body.into()).await;

    assert_eq!(details.in_production, Some(false));
}

#[tokio::test]
async fn tv_series_details_in_production_true_is_some_true() {
    let mut body = tv_details_json();
    body.insert("in_production".to_string(), serde_json::Value::Bool(true));

    let details = tv_details_with_body(body.into()).await;

    assert_eq!(details.in_production, Some(true));
}
