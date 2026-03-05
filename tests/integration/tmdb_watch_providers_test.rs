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

    let resp = client.movie_watch_providers(550).await.unwrap();
    let body = resp.into_inner();

    assert_eq!(body.id, 550);
    assert!(body.results.is_some());
}
