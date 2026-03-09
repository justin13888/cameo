use cameo::providers::tmdb::{TmdbClient, TmdbConfig, TmdbError};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path},
};

async fn setup_tmdb_client() -> (MockServer, TmdbClient) {
    let server = MockServer::start().await;
    let config = TmdbConfig::new_with_base_url("test-token", server.uri());
    let client = TmdbClient::new(config).unwrap();
    (server, client)
}

#[tokio::test]
async fn http_401_returns_api_error_with_status() {
    let (server, client) = setup_tmdb_client().await;

    Mock::given(method("GET"))
        .and(path("/3/movie/550"))
        .respond_with(ResponseTemplate::new(401).set_body_raw(
            r#"{"status_code":7,"status_message":"Invalid API key."}"#,
            "application/json",
        ))
        .mount(&server)
        .await;

    let err = client.movie_details(550).await.unwrap_err();

    match err {
        TmdbError::Api { status, .. } => assert_eq!(status, 401),
        other => panic!("expected Api error, got {other:?}"),
    }
}

#[tokio::test]
async fn http_404_returns_api_error_with_status() {
    let (server, client) = setup_tmdb_client().await;

    Mock::given(method("GET"))
        .and(path("/3/movie/999999999"))
        .respond_with(ResponseTemplate::new(404).set_body_raw(
            r#"{"status_code":34,"status_message":"The resource you requested could not be found."}"#,
            "application/json",
        ))
        .mount(&server)
        .await;

    let err = client.movie_details(999999999).await.unwrap_err();

    match err {
        TmdbError::Api { status, .. } => assert_eq!(status, 404),
        other => panic!("expected Api error, got {other:?}"),
    }
}

#[tokio::test]
async fn http_429_returns_api_error_with_status() {
    let (server, client) = setup_tmdb_client().await;

    Mock::given(method("GET"))
        .and(path("/3/movie/550"))
        .respond_with(ResponseTemplate::new(429).set_body_raw(
            r#"{"status_code":25,"status_message":"Your request count is over the allowed limit."}"#,
            "application/json",
        ))
        .mount(&server)
        .await;

    let err = client.movie_details(550).await.unwrap_err();

    match err {
        TmdbError::Api { status, .. } => assert_eq!(status, 429),
        other => panic!("expected Api error, got {other:?}"),
    }
}

#[tokio::test]
async fn http_500_returns_api_error_with_status() {
    let (server, client) = setup_tmdb_client().await;

    Mock::given(method("GET"))
        .and(path("/3/movie/550"))
        .respond_with(ResponseTemplate::new(500).set_body_raw(
            r#"{"status_code":11,"status_message":"Internal error."}"#,
            "application/json",
        ))
        .mount(&server)
        .await;

    let err = client.movie_details(550).await.unwrap_err();

    match err {
        TmdbError::Api { status, .. } => assert_eq!(status, 500),
        other => panic!("expected Api error, got {other:?}"),
    }
}

#[tokio::test]
async fn malformed_json_returns_deserialization_error() {
    let (server, client) = setup_tmdb_client().await;

    Mock::given(method("GET"))
        .and(path("/3/movie/550"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw("this is not json {{{{", "application/json"),
        )
        .mount(&server)
        .await;

    let err = client.movie_details(550).await.unwrap_err();

    assert!(
        matches!(err, TmdbError::Api { .. } | TmdbError::Deserialization(_)),
        "expected Api or Deserialization error, got {err:?}"
    );
}

#[tokio::test]
async fn rate_limit_timeout_fires() {
    use std::time::Duration;

    // A semaphore with 0 permits means no requests can proceed.
    let server = MockServer::start().await;
    let config = TmdbConfig::new_with_base_url("test-token", server.uri())
        .with_rate_limit(0) // 0 permits → always blocks
        .with_rate_limit_timeout(Duration::from_millis(50));
    let client = TmdbClient::new(config).unwrap();

    let err = client.movie_details(550).await.unwrap_err();

    assert!(
        matches!(err, TmdbError::RateLimitExceeded),
        "expected RateLimitExceeded, got {err:?}"
    );
}
