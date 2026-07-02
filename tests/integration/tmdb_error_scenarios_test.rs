use std::time::Duration;

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
async fn http_401_decodes_status_code_and_message() {
    let (server, client) = setup_tmdb_client().await;

    Mock::given(method("GET"))
        .and(path("/3/movie/550"))
        .respond_with(ResponseTemplate::new(401).set_body_raw(
            r#"{"status_code":7,"status_message":"Invalid API key: You must be granted a valid key."}"#,
            "application/json",
        ))
        .mount(&server)
        .await;

    let err = client.movie_details(550).await.unwrap_err();

    match err {
        TmdbError::Api {
            status,
            status_code,
            message,
            ..
        } => {
            assert_eq!(status, 401);
            assert_eq!(status_code, Some(7));
            assert!(message.contains("Invalid API key"), "got: {message}");
        }
        other => panic!("expected Api error, got {other:?}"),
    }
}

#[tokio::test]
async fn http_404_decodes_status_code_and_message() {
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
        TmdbError::Api {
            status,
            status_code,
            message,
            ..
        } => {
            assert_eq!(status, 404);
            assert_eq!(status_code, Some(34));
            assert!(message.contains("could not be found"), "got: {message}");
        }
        other => panic!("expected Api error, got {other:?}"),
    }
}

#[tokio::test]
async fn http_429_decodes_message_and_retry_after() {
    let (server, client) = setup_tmdb_client().await;

    Mock::given(method("GET"))
        .and(path("/3/movie/550"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("Retry-After", "7")
                .set_body_raw(
                    r#"{"status_code":25,"status_message":"Your request count is over the allowed limit."}"#,
                    "application/json",
                ),
        )
        .mount(&server)
        .await;

    let err = client.movie_details(550).await.unwrap_err();

    match err {
        TmdbError::Api {
            status,
            status_code,
            retry_after,
            message,
        } => {
            assert_eq!(status, 429);
            assert_eq!(status_code, Some(25));
            assert_eq!(retry_after, Some(Duration::from_secs(7)));
            assert!(message.contains("over the allowed limit"), "got: {message}");
        }
        other => panic!("expected Api error, got {other:?}"),
    }
}

#[tokio::test]
async fn http_500_maps_to_api_error() {
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

/// A malformed body on a 2xx response is a *deserialization* failure, distinct
/// from an API-status error.
#[tokio::test]
async fn malformed_json_body_is_deserialization_error() {
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
        matches!(err, TmdbError::Deserialization(_)),
        "expected Deserialization error, got {err:?}"
    );
}

/// A connection failure (nothing listening) is a *transport* error, distinct
/// from an API-status error.
#[tokio::test]
async fn connection_failure_is_transport_error() {
    // Point at a port that is (almost certainly) closed.
    let config = TmdbConfig::new_with_base_url("test-token", "http://127.0.0.1:1");
    let client = TmdbClient::new(config).unwrap();

    let err = client.movie_details(550).await.unwrap_err();

    assert!(
        matches!(err, TmdbError::Http(_)),
        "expected Http transport error, got {err:?}"
    );
}

#[tokio::test]
async fn rate_limit_timeout_fires() {
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
