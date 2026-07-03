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
async fn movie_recommendations_deserializes_response() {
    let (server, client) = setup_mock_client().await;

    Mock::given(method("GET"))
        .and(path("/3/movie/550/recommendations"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            include_str!("../fixtures/movie_recommendations_response.json"),
            "application/json",
        ))
        .mount(&server)
        .await;

    let page = client.movie_recommendations(550, None).await.unwrap();

    assert_eq!(page.page, 1);
    assert_eq!(page.total_results, 40);
    assert_eq!(page.results.len(), 1);
    assert_eq!(page.results[0].provider_id, "tmdb:9300");
    assert_eq!(page.results[0].title, "Orlando");
}

#[tokio::test]
async fn tv_recommendations_deserializes_response() {
    let (server, client) = setup_mock_client().await;

    Mock::given(method("GET"))
        .and(path("/3/tv/1399/recommendations"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            include_str!("../fixtures/tv_recommendations_response.json"),
            "application/json",
        ))
        .mount(&server)
        .await;

    let page = client.tv_recommendations(1399, None).await.unwrap();

    assert_eq!(page.page, 1);
    assert_eq!(page.total_results, 40);
    assert_eq!(page.results.len(), 1);
    assert_eq!(page.results[0].provider_id, "tmdb:1396");
    assert_eq!(page.results[0].name, "Breaking Bad");
}
