use std::time::Duration;

use cameo::{
    into_stream,
    providers::tmdb::{TmdbClient, TmdbConfig},
};
use futures::StreamExt;
use serde_json::json;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path, query_param},
};

async fn setup_mock_client() -> (MockServer, TmdbClient) {
    let server = MockServer::start().await;
    let client =
        TmdbClient::new(TmdbConfig::new_with_base_url("test-token", server.uri())).unwrap();
    (server, client)
}

#[tokio::test]
async fn tv_popular_list_deserializes_response() {
    let (server, client) = setup_mock_client().await;

    Mock::given(method("GET"))
        .and(path("/3/tv/popular"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            include_str!("../fixtures/tv_popular_response.json"),
            "application/json",
        ))
        .mount(&server)
        .await;

    let page = client.popular_tv_shows(None).await.unwrap();

    assert_eq!(page.page, 1);
    assert_eq!(page.total_results, 200);
    assert_eq!(page.results.len(), 1);
    assert_eq!(page.results[0].provider_id, "tmdb:202250");
    assert_eq!(page.results[0].name, "Dirty Linen");
}

/// Trending ignores the page argument, so a body claiming more pages still
/// yields a single page, and a stream over it ends after one request.
#[tokio::test]
async fn trending_tv_is_a_single_page_and_its_stream_terminates() {
    let (server, client) = setup_mock_client().await;

    Mock::given(method("GET"))
        .and(path("/3/trending/tv/week"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "page": 1,
            "results": [{ "id": 1396, "name": "Breaking Bad" }],
            "total_pages": 1000,
            "total_results": 20_000,
        })))
        .expect(2)
        .mount(&server)
        .await;

    let page = client.trending_tv(cameo::TimeWindow::Week).await.unwrap();
    assert!(!page.has_next);
    assert_eq!(page.next, None);
    assert_eq!(page.total_pages, 1);
    assert_eq!(page.total_results, 20_000);

    let stream = into_stream(move |_page| {
        let client = client.clone();
        async move { client.trending_tv(cameo::TimeWindow::Week).await }
    });
    let items: Vec<_> = tokio::time::timeout(Duration::from_secs(60), stream.collect::<Vec<_>>())
        .await
        .expect("trending stream terminates");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].as_ref().unwrap().provider_id, "tmdb:1396");
}

/// The discover builder's page 500 of a longer result set is the last page.
#[tokio::test]
async fn discover_tv_page_500_is_the_last_page() {
    let (server, client) = setup_mock_client().await;

    Mock::given(method("GET"))
        .and(path("/3/discover/tv"))
        .and(query_param("page", "500"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "page": 500,
            "results": [{ "id": 1396, "name": "Breaking Bad" }],
            "total_pages": 1000,
            "total_results": 20_000,
        })))
        .mount(&server)
        .await;

    let page = client.discover_tv().page(500).execute().await.unwrap();

    assert_eq!(page.page, 500);
    assert!(!page.has_next);
    assert_eq!(page.next, None);
    assert_eq!(page.total_pages, 500);
}

#[tokio::test]
async fn tv_top_rated_list_deserializes_response() {
    let (server, client) = setup_mock_client().await;

    Mock::given(method("GET"))
        .and(path("/3/tv/top_rated"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            include_str!("../fixtures/tv_top_rated_response.json"),
            "application/json",
        ))
        .mount(&server)
        .await;

    let page = client.top_rated_tv_shows(None).await.unwrap();

    assert_eq!(page.page, 1);
    assert_eq!(page.total_results, 100);
    assert_eq!(page.results.len(), 1);
    assert_eq!(page.results[0].provider_id, "tmdb:130392");
    assert_eq!(page.results[0].name, "The D'Amelio Show");
}
