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

    let resp = client.tv_series_popular_list(None, None).await.unwrap();
    let body = resp.into_inner();

    assert_eq!(body.page, 1);
    assert_eq!(body.total_results, 200);
    assert_eq!(body.results.len(), 1);
    assert_eq!(body.results[0].id, 202250);
    assert_eq!(body.results[0].name.as_deref(), Some("Dirty Linen"));
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

    let resp = client.tv_series_top_rated_list(None, None).await.unwrap();
    let body = resp.into_inner();

    assert_eq!(body.page, 1);
    assert_eq!(body.total_results, 100);
    assert_eq!(body.results.len(), 1);
    assert_eq!(body.results[0].id, 130392);
    assert_eq!(body.results[0].name.as_deref(), Some("The D'Amelio Show"));
}
