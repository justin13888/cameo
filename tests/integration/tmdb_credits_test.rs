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
async fn movie_credits_deserializes_response() {
    let (server, client) = setup_mock_client().await;

    Mock::given(method("GET"))
        .and(path("/3/movie/550/credits"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            include_str!("../fixtures/movie_credits_response.json"),
            "application/json",
        ))
        .mount(&server)
        .await;

    let resp = client.movie_credits(550, None).await.unwrap();
    let body = resp.into_inner();

    assert_eq!(body.id, 550);
    assert_eq!(body.cast.len(), 1);
    assert_eq!(body.cast[0].name.as_deref(), Some("Edward Norton"));
    assert_eq!(body.cast[0].character.as_deref(), Some("The Narrator"));
    assert_eq!(body.crew.len(), 1);
    assert_eq!(body.crew[0].name.as_deref(), Some("David Fincher"));
    assert_eq!(body.crew[0].job.as_deref(), Some("Director"));
}

#[tokio::test]
async fn tv_series_credits_deserializes_response() {
    let (server, client) = setup_mock_client().await;

    Mock::given(method("GET"))
        .and(path("/3/tv/1396/aggregate_credits"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            include_str!("../fixtures/tv_credits_response.json"),
            "application/json",
        ))
        .mount(&server)
        .await;

    let resp = client
        .tv_series_aggregate_credits(1396, None)
        .await
        .unwrap();
    let body = resp.into_inner();

    assert_eq!(body.id, 1396);
    assert_eq!(body.cast.len(), 1);
    assert_eq!(body.cast[0].name.as_deref(), Some("Bryan Cranston"));
    assert_eq!(body.cast[0].total_episode_count, 62);
    assert_eq!(body.crew.len(), 1);
    assert_eq!(body.crew[0].name.as_deref(), Some("Vince Gilligan"));
}
