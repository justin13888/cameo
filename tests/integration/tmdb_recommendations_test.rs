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

    let resp = client.movie_recommendations(550, None, None).await.unwrap();
    let map = resp.into_inner();

    assert_eq!(map["page"].as_i64(), Some(1));
    assert_eq!(map["total_results"].as_i64(), Some(40));
    let results = map["results"].as_array().unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0]["id"].as_i64(), Some(9300));
    assert_eq!(results[0]["title"].as_str(), Some("Orlando"));
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

    let resp = client
        .tv_series_recommendations(1399, None, None)
        .await
        .unwrap();
    let body = resp.into_inner();

    assert_eq!(body.page, 1);
    assert_eq!(body.total_results, 40);
    assert_eq!(body.results.len(), 1);
    assert_eq!(body.results[0].id, 1396);
    assert_eq!(body.results[0].name.as_deref(), Some("Breaking Bad"));
}
