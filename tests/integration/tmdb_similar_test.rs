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

    let resp = client.movie_similar(550, None, None).await.unwrap();
    let body = resp.into_inner();

    assert_eq!(body.page, 1);
    assert_eq!(body.total_results, 40);
    assert_eq!(body.results.len(), 1);
    assert_eq!(body.results[0].id, 9300);
    assert_eq!(body.results[0].title.as_deref(), Some("Orlando"));
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

    let resp = client.tv_series_similar("1396", None, None).await.unwrap();
    let body = resp.into_inner();

    assert_eq!(body.page, 1);
    assert_eq!(body.total_results, 1);
    assert_eq!(body.results.len(), 1);
    assert_eq!(body.results[0].id, 202250);
    assert_eq!(body.results[0].name.as_deref(), Some("Dirty Linen"));
}
