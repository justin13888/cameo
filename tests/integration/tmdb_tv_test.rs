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
