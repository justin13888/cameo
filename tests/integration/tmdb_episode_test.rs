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

    let resp = client
        .tv_episode_details(1396, 1, 1, None, None)
        .await
        .unwrap();
    let body = resp.into_inner();

    assert_eq!(body.id, 62085);
    assert_eq!(body.episode_number, 1);
    assert_eq!(body.season_number, 1);
    assert_eq!(body.name.as_deref(), Some("Pilot"));
    assert_eq!(body.runtime, 58);
}
