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
async fn movie_watch_providers_deserializes_response() {
    let (server, client) = setup_mock_client().await;

    Mock::given(method("GET"))
        .and(path("/3/movie/550/watch/providers"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            include_str!("../fixtures/movie_watch_providers_response.json"),
            "application/json",
        ))
        .mount(&server)
        .await;

    let providers = client.movie_watch_providers(550).await.unwrap();

    assert!(providers.provider_id == "tmdb:550");
    let us = providers
        .results
        .get("US")
        .expect("US watch providers present");
    assert_eq!(us.flatrate[0].name, "fuboTV");
    assert_eq!(us.rent[0].name, "Amazon Video");
    assert_eq!(us.buy[0].name, "Apple TV");
}

#[tokio::test]
async fn tv_watch_providers_deserializes_response() {
    let (server, client) = setup_mock_client().await;

    Mock::given(method("GET"))
        .and(path("/3/tv/1396/watch/providers"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            include_str!("../fixtures/tv_watch_providers_response.json"),
            "application/json",
        ))
        .mount(&server)
        .await;

    let providers = client.tv_watch_providers(1396).await.unwrap();

    assert!(providers.provider_id == "tmdb:1396");
    let us = providers
        .results
        .get("US")
        .expect("US watch providers present");
    assert_eq!(us.flatrate[0].name, "Netflix");
    assert!(us.rent.is_empty());
    assert_eq!(us.buy[0].name, "Apple TV");
}
