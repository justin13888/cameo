//! The retry loop.

use super::*;

/// Call `movie_details` against a server answering `template`, with two
/// fast retries, asserting the server is hit exactly `attempts` times.
async fn attempts_for(template: ResponseTemplate, attempts: u64) -> AniListError {
    let server = serve(template, attempts).await;
    let client = client_with(&server, |c| c.with_retry(TWO_FAST_RETRIES));
    let err = client.movie_details(5).await.unwrap_err();
    drop(server); // verifies the attempt count
    err
}

#[tokio::test]
async fn rate_limit_and_server_errors_are_retried_until_the_policy_gives_up() {
    let rate_limited = json_response(
        429,
        json!({ "errors": [{ "message": "Too Many Requests.", "status": 429 }] }),
    )
    .insert_header("Retry-After", "0");
    let err = attempts_for(rate_limited, 3).await;
    assert_eq!(graphql_errors(&err)[0].status, Some(429));

    let err = attempts_for(raw_response(503, "unavailable"), 3).await;
    assert!(
        matches!(err, AniListError::Status { status: 503, .. }),
        "got {err:?}"
    );

    let gql_5xx = json_response(
        200,
        json!({ "data": { "Media": null }, "errors": [{ "message": "boom", "status": 502 }] }),
    );
    let err = attempts_for(gql_5xx, 3).await;
    assert_eq!(graphql_errors(&err)[0].status, Some(502));
}

#[tokio::test]
async fn non_transient_errors_are_not_retried() {
    let bad_request = json_response(
        200,
        json!({ "errors": [{ "message": "x", "status": 400 }] }),
    );
    assert!(matches!(
        attempts_for(bad_request, 1).await,
        AniListError::GraphQL { .. }
    ));
    let not_found = json_response(404, json!({ "errors": [{ "message": "Not Found." }] }));
    assert!(matches!(
        attempts_for(not_found, 1).await,
        AniListError::NotFound
    ));
    assert!(matches!(
        attempts_for(raw_response(200, "not json"), 1).await,
        AniListError::Deserialization(_)
    ));
    assert!(matches!(
        attempts_for(raw_response(404, "gone"), 1).await,
        AniListError::Status { status: 404, .. }
    ));
}

#[tokio::test]
async fn a_disabled_retry_policy_makes_one_attempt() {
    let server = serve(raw_response(503, "unavailable"), 1).await;
    let err = client_for(&server).movie_details(5).await.unwrap_err();
    assert!(
        matches!(err, AniListError::Status { status: 503, .. }),
        "got {err:?}"
    );
}

#[tokio::test]
async fn a_retry_recovers_with_the_next_success() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(raw_response(503, "unavailable"))
        .up_to_n_times(1)
        .with_priority(1)
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .respond_with(json_response(200, details()))
        .with_priority(2)
        .expect(1)
        .mount(&server)
        .await;
    let client = client_with(&server, |c| c.with_retry(TWO_FAST_RETRIES));
    let movie = client.movie_details(5).await.expect("recovers");
    assert_eq!(movie.movie.provider_id, "anilist:5");
    // Every attempt sends the same request.
    let sent = requests(&server).await;
    assert_eq!(body_of(&sent[0]), body_of(&sent[1]));
}

#[tokio::test]
async fn every_attempt_acquires_the_rate_limiter() {
    const WINDOW: Duration = Duration::from_millis(300);
    let server = serve(raw_response(503, "unavailable"), 3).await;
    // One request per window: the first attempt spends the only token, so
    // each of the two retries waits for a refill.
    let client = client_with(&server, |c| {
        c.with_retry(TWO_FAST_RETRIES)
            .with_rate_limit(RateLimit::per(1, WINDOW))
    });
    let started = Instant::now();
    let err = client.movie_details(5).await.unwrap_err();
    let elapsed = started.elapsed();
    assert!(matches!(err, AniListError::Status { status: 503, .. }));
    // Backoff is ~1ms per retry, so only the limiter explains the wait.
    assert!(
        elapsed >= WINDOW * 2 - Duration::from_millis(20),
        "three attempts took only {elapsed:?}"
    );
}

/// A details response with an invalid id is converted under the
/// requested id instead of a wrapped one.
#[tokio::test]
async fn movie_details_with_invalid_reported_id_uses_requested_id() {
    let server = serve(
        ResponseTemplate::new(200).set_body_json(json!({
            "data": { "Media": { "id": -1, "format": "MOVIE" } }
        })),
        1,
    )
    .await;
    let client = client_with(&server, |c| c);
    let movie = client.movie_details(5).await.expect("details");
    assert_eq!(movie.movie.provider_id, "anilist:5");
}
