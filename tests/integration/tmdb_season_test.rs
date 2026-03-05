use cameo::generated::tmdb::Client as GeneratedClient;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path},
};

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

    let resp = client.tv_season_details(1399, 1, None, None).await.unwrap();
    let body = resp.into_inner();

    assert_eq!(body.season_number, 1);
    assert_eq!(body.name.as_deref(), Some("Season 1"));
    assert_eq!(body.episodes.len(), 1);

    let ep = &body.episodes[0];
    assert_eq!(ep.episode_number, 1);
    assert_eq!(ep.name.as_deref(), Some("Winter Is Coming"));
    assert_eq!(ep.runtime, 62);
}
