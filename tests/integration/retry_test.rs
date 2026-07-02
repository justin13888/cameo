//! Retry behavior: transient 429s recover, and give-up is bounded.

use std::time::Duration;

use cameo::RetryPolicy;
use wiremock::{Mock, MockServer, ResponseTemplate, matchers::method};

const FAST_RETRY: RetryPolicy =
    RetryPolicy::new(3, Duration::from_millis(20), Duration::from_millis(100));

// ── TMDB ──────────────────────────────────────────────────────────────────────

#[cfg(feature = "tmdb")]
mod tmdb {
    use cameo::providers::tmdb::{TmdbClient, TmdbConfig, TmdbError};
    use wiremock::matchers::path;

    use super::*;

    const EMPTY_SEARCH: &str = r#"{"page":1,"results":[],"total_pages":1,"total_results":0}"#;

    #[tokio::test]
    async fn retries_429_then_succeeds() {
        let server = MockServer::start().await;
        // First response: 429 (matches once, higher priority).
        Mock::given(method("GET"))
            .and(path("/3/search/movie"))
            .respond_with(
                ResponseTemplate::new(429)
                    .insert_header("Retry-After", "0")
                    .set_body_raw(
                        r#"{"status_code":25,"status_message":"limit"}"#,
                        "application/json",
                    ),
            )
            .up_to_n_times(1)
            .with_priority(1)
            .mount(&server)
            .await;
        // Subsequent responses: success.
        Mock::given(method("GET"))
            .and(path("/3/search/movie"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(EMPTY_SEARCH, "application/json"))
            .with_priority(2)
            .mount(&server)
            .await;

        let config = TmdbConfig::new_with_base_url("t", server.uri()).with_retry(FAST_RETRY);
        let client = TmdbClient::new(config).unwrap();

        let result = client.search_movies("x", None).await;
        assert!(
            result.is_ok(),
            "expected recovery after 429, got {result:?}"
        );
    }

    #[tokio::test]
    async fn gives_up_after_bounded_retries() {
        let server = MockServer::start().await;
        // Always 429. With max_retries = 2 the client makes exactly 3 attempts.
        Mock::given(method("GET"))
            .and(path("/3/search/movie"))
            .respond_with(ResponseTemplate::new(429).set_body_raw(
                r#"{"status_code":25,"status_message":"limit"}"#,
                "application/json",
            ))
            .expect(3)
            .mount(&server)
            .await;

        let policy = RetryPolicy::new(2, Duration::from_millis(10), Duration::from_millis(20));
        let config = TmdbConfig::new_with_base_url("t", server.uri()).with_retry(policy);
        let client = TmdbClient::new(config).unwrap();

        let err = client.search_movies("x", None).await.unwrap_err();
        assert!(
            matches!(err, TmdbError::Api { status: 429, .. }),
            "expected 429 after give-up, got {err:?}"
        );
        // `expect(3)` is verified on server drop: exactly 3 attempts were made.
    }

    #[tokio::test]
    async fn disabled_policy_does_not_retry() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/3/search/movie"))
            .respond_with(ResponseTemplate::new(503).set_body_raw(
                r#"{"status_code":11,"status_message":"err"}"#,
                "application/json",
            ))
            .expect(1)
            .mount(&server)
            .await;

        let config =
            TmdbConfig::new_with_base_url("t", server.uri()).with_retry(RetryPolicy::disabled());
        let client = TmdbClient::new(config).unwrap();

        let err = client.search_movies("x", None).await.unwrap_err();
        assert!(
            matches!(err, TmdbError::Api { status: 503, .. }),
            "got {err:?}"
        );
    }
}

// ── AniList ───────────────────────────────────────────────────────────────────

#[cfg(feature = "anilist")]
mod anilist {
    use cameo::providers::anilist::{AniListClient, AniListConfig};

    use super::*;

    fn empty_page() -> serde_json::Value {
        serde_json::json!({
            "data": { "Page": {
                "pageInfo": { "total": 0, "currentPage": 1, "lastPage": 1, "hasNextPage": false, "perPage": 20 },
                "media": []
            }}
        })
    }

    #[tokio::test]
    async fn retries_429_then_succeeds() {
        let server = MockServer::start().await;
        // AniList signals rate limiting as a GraphQL error with status 429.
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(429)
                    .insert_header("Retry-After", "0")
                    .set_body_json(serde_json::json!({
                        "errors": [{ "message": "Too Many Requests.", "status": 429 }]
                    })),
            )
            .up_to_n_times(1)
            .with_priority(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(empty_page()))
            .with_priority(2)
            .mount(&server)
            .await;

        let config = AniListConfig::new_with_base_url(server.uri()).with_retry(FAST_RETRY);
        let client = AniListClient::new(config).unwrap();

        let result = client.search_tv_shows("x", None).await;
        assert!(
            result.is_ok(),
            "expected recovery after 429, got {result:?}"
        );
    }
}
