use std::time::Duration;

use cameo::{
    RetryPolicy,
    providers::tmdb::{TmdbClient, TmdbConfig, TmdbError},
};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path},
};

/// Error decoding is tested with retries disabled, so each scenario is a
/// single attempt; attempt counts under retry are pinned in `retry_test.rs`.
async fn setup_tmdb_client() -> (MockServer, TmdbClient) {
    let server = MockServer::start().await;
    let config = TmdbConfig::new_with_base_url("test-token", server.uri())
        .with_retry(RetryPolicy::disabled());
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
        .expect(1)
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
        .expect(1)
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
        .expect(1)
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
        .expect(1)
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
        .expect(1)
        .mount(&server)
        .await;

    let err = client.movie_details(550).await.unwrap_err();

    assert!(
        matches!(err, TmdbError::Deserialization(_)),
        "expected Deserialization error, got {err:?}"
    );
    // The payload is cameo's own DecodeError, reachable through `source()`.
    let decode = std::error::Error::source(&err)
        .and_then(|e| e.downcast_ref::<cameo::DecodeError>())
        .unwrap_or_else(|| panic!("no DecodeError source: {err:?}"));
    assert!(!decode.to_string().is_empty(), "{decode:?}");
}

/// A server that accepts the connection, reads the request, and closes it
/// without responding is a *transport* error (no HTTP response was received),
/// never an API error, and maps to `ProviderError::Transport`.
#[tokio::test]
async fn connection_closed_without_response_is_transport_error() {
    use std::io::Read;

    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut buf = [0u8; 4096];
        let _ = stream.read(&mut buf);
        // Dropping the stream closes the connection with no response.
    });

    let config = TmdbConfig::new_with_base_url("test-token", format!("http://{addr}"))
        .with_retry(RetryPolicy::disabled());
    let client = TmdbClient::new(config).unwrap();

    let err = client.movie_details(550).await.unwrap_err();
    server.join().unwrap();

    match &err {
        TmdbError::Http(e) => assert!(!e.is_connect() && !e.is_timeout(), "{e:?}"),
        other => panic!("expected Http transport error, got {other:?}"),
    }
    assert!(
        matches!(
            cameo::ProviderError::from(err),
            cameo::ProviderError::Transport(_)
        ),
        "expected ProviderError::Transport"
    );
}

/// A request that cannot be built is a *request* error: nothing is sent, it is
/// never an API error with a made-up status, and it maps to
/// `ProviderError::InvalidInput`.
#[tokio::test]
async fn unbuildable_request_is_invalid_request_error() {
    use cameo::providers::tmdb::RequestErrorKind;

    let (server, client) = setup_tmdb_client().await;

    // RFC 3339 `full-date` years are 0000..=9999; jiff also holds negative years.
    let out_of_range = jiff::civil::date(-1, 1, 1);
    let err = client
        .discover_movies()
        .primary_release_date_gte(out_of_range)
        .execute()
        .await
        .unwrap_err();

    assert!(
        server.received_requests().await.unwrap().is_empty(),
        "nothing should have been sent"
    );
    assert!(
        matches!(
            err,
            TmdbError::Request {
                kind: RequestErrorKind::InvalidRequest,
                ..
            }
        ),
        "expected Request error, got {err:?}"
    );
    assert!(
        matches!(
            cameo::ProviderError::from(err),
            cameo::ProviderError::InvalidInput(_)
        ),
        "expected ProviderError::InvalidInput"
    );
}

/// Whether `result` failed with an invalid-request error.
fn is_invalid_request<T>(result: &Result<T, TmdbError>) -> bool {
    use cameo::providers::tmdb::RequestErrorKind;
    matches!(
        result,
        Err(TmdbError::Request {
            kind: RequestErrorKind::InvalidRequest,
            ..
        })
    )
}

/// An id, season, or episode number above `i32::MAX` is rejected by every
/// id-taking method before anything is sent, rather than wrapped or clamped.
#[tokio::test]
async fn out_of_range_ids_are_rejected_before_sending() {
    let (server, client) = setup_tmdb_client().await;
    let big = u64::try_from(i32::MAX).expect("positive") + 1;
    let big_number = u32::try_from(big).expect("fits u32");

    assert!(is_invalid_request(&client.movie_details(big).await));
    assert!(is_invalid_request(
        &client.movie_details_with_append(big, "credits").await
    ));
    assert!(is_invalid_request(&client.tv_show_details(big).await));
    assert!(is_invalid_request(
        &client.tv_show_details_with_append(big, "credits").await
    ));
    assert!(is_invalid_request(&client.person_details(big).await));
    assert!(is_invalid_request(&client.movie_credits(big).await));
    assert!(is_invalid_request(&client.tv_show_credits(big).await));
    assert!(is_invalid_request(&client.movie_images(big).await));
    assert!(is_invalid_request(
        &client.movie_recommendations(big, None).await
    ));
    assert!(is_invalid_request(
        &client.tv_recommendations(big, None).await
    ));
    assert!(is_invalid_request(&client.similar_movies(big, None).await));
    assert!(is_invalid_request(
        &client.similar_tv_shows(big, None).await
    ));
    assert!(is_invalid_request(&client.movie_watch_providers(big).await));
    assert!(is_invalid_request(&client.tv_watch_providers(big).await));
    assert!(is_invalid_request(&client.movie_external_ids(big).await));
    assert!(is_invalid_request(&client.tv_external_ids(big).await));
    assert!(is_invalid_request(&client.season_details(big, 1).await));
    assert!(is_invalid_request(
        &client.season_details(1399, big_number).await
    ));
    assert!(is_invalid_request(&client.episode_details(big, 1, 1).await));
    assert!(is_invalid_request(
        &client.episode_details(1399, big_number, 1).await
    ));
    assert!(is_invalid_request(
        &client.episode_details(1399, 1, big_number).await
    ));
    assert!(is_invalid_request(
        &client.discover_tv().with_networks(big).execute().await
    ));

    assert!(
        server.received_requests().await.unwrap().is_empty(),
        "nothing should have been sent"
    );
}

/// Through the unified traits, an out-of-range TMDB id surfaces as
/// `ProviderError::InvalidInput`, as it did before the inherent methods took
/// `u64`.
#[tokio::test]
async fn out_of_range_media_id_is_invalid_input() {
    use cameo::{DetailProvider, MediaId, ProviderError};

    let (server, client) = setup_tmdb_client().await;
    let big = u64::try_from(i32::MAX).expect("positive") + 1;

    let err = DetailProvider::movie_details(&client, &MediaId::tmdb(big))
        .await
        .unwrap_err();

    assert!(
        matches!(err, ProviderError::InvalidInput(_)),
        "expected InvalidInput, got {err:?}"
    );
    assert!(
        server.received_requests().await.unwrap().is_empty(),
        "nothing should have been sent"
    );
}

/// A connection failure (nothing listening) is a *transport* error, distinct
/// from an API-status error.
#[tokio::test]
async fn connection_failure_is_transport_error() {
    // Point at a port that is (almost certainly) closed.
    let config = TmdbConfig::new_with_base_url("test-token", "http://127.0.0.1:1")
        .with_retry(RetryPolicy::disabled());
    let client = TmdbClient::new(config).unwrap();

    let err = client.movie_details(550).await.unwrap_err();

    assert!(
        matches!(err, TmdbError::Http(_)),
        "expected Http transport error, got {err:?}"
    );
}
