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
async fn tv_episode_details_deserializes_response() {
    let (server, client) = setup_mock_client().await;

    Mock::given(method("GET"))
        .and(path("/3/tv/1396/season/1/episode/1"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            include_str!("../fixtures/tv_episode_details_response.json"),
            "application/json",
        ))
        .mount(&server)
        .await;

    let ep = client.tv_episode_details(1396, 1, 1).await.unwrap();

    assert_eq!(ep.episode_number, 1);
    assert_eq!(ep.name.as_deref(), Some("Pilot"));
    assert_eq!(ep.runtime, Some(58));
    assert_eq!(ep.vote_average, Some(7.7));
}
