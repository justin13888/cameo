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
async fn tv_season_details_deserializes_response() {
    let (server, client) = setup_mock_client().await;

    Mock::given(method("GET"))
        .and(path("/3/tv/1399/season/1"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            include_str!("../fixtures/tv_season_details_response.json"),
            "application/json",
        ))
        .mount(&server)
        .await;

    let season = client.tv_season_details(1399, 1).await.unwrap();

    assert_eq!(season.show_id, "tmdb:1399");
    assert_eq!(season.season_number, 1);
    assert_eq!(season.name.as_deref(), Some("Season 1"));
    assert_eq!(season.episodes.len(), 1);

    let ep = &season.episodes[0];
    assert_eq!(ep.episode_number, 1);
    assert_eq!(ep.name.as_deref(), Some("Winter Is Coming"));
    assert_eq!(ep.runtime, Some(62));
}
