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
async fn similar_movies_deserializes_response() {
    let (server, client) = setup_mock_client().await;

    Mock::given(method("GET"))
        .and(path("/3/movie/550/similar"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            include_str!("../fixtures/similar_movies_response.json"),
            "application/json",
        ))
        .mount(&server)
        .await;

    let page = client.similar_movies(550, None).await.unwrap();

    assert_eq!(page.page, 1);
    assert_eq!(page.total_results, 40);
    assert_eq!(page.results.len(), 1);
    assert_eq!(page.results[0].provider_id, "tmdb:9300");
    assert_eq!(page.results[0].title, "Orlando");
}

#[tokio::test]
async fn similar_tv_shows_deserializes_response() {
    let (server, client) = setup_mock_client().await;

    Mock::given(method("GET"))
        .and(path("/3/tv/1396/similar"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            include_str!("../fixtures/similar_tv_response.json"),
            "application/json",
        ))
        .mount(&server)
        .await;

    let page = client.similar_tv_shows(1396, None).await.unwrap();

    assert_eq!(page.page, 1);
    assert_eq!(page.total_results, 1);
    assert_eq!(page.results.len(), 1);
    assert_eq!(page.results[0].provider_id, "tmdb:202250");
    assert_eq!(page.results[0].name, "Dirty Linen");
}
