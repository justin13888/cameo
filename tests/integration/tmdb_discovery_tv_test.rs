use cameo::providers::tmdb::{TmdbClient, TmdbConfig};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path},
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
