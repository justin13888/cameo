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
            .expect(1)
            .mount(&server)
            .await;
        // Subsequent responses: success.
        Mock::given(method("GET"))
            .and(path("/3/search/movie"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(EMPTY_SEARCH, "application/json"))
            .with_priority(2)
            .expect(1)
            .mount(&server)
            .await;

        let config = TmdbConfig::new_with_base_url("t", server.uri()).with_retry(FAST_RETRY);
        let client = TmdbClient::new(config).unwrap();

        // One 429, then one success: exactly two attempts (verified on drop).
        let page = client
            .search_movies("x", None)
            .await
            .expect("recovers after one 429");
        assert_eq!(page.page, 1);
        assert_eq!(page.total_pages, 1);
        assert_eq!(page.total_results, 0);
        assert!(page.results.is_empty());
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

    /// A 2-retry policy with tiny backoff, so give-up tests stay fast.
    const TWO_RETRIES: RetryPolicy =
        RetryPolicy::new(2, Duration::from_millis(1), Duration::from_millis(2));

    /// Serve `template` for every `/3/search/movie` request, asserting it is
    /// hit exactly `expected` times, and build a client from the config
    /// `configure` returns.
    async fn search_server(
        template: ResponseTemplate,
        expected: u64,
        configure: impl FnOnce(TmdbConfig) -> TmdbConfig,
    ) -> (MockServer, TmdbClient) {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/3/search/movie"))
            .respond_with(template)
            .expect(expected)
            .mount(&server)
            .await;
        let config = configure(TmdbConfig::new_with_base_url("t", server.uri()));
        (server, TmdbClient::new(config).unwrap())
    }

    #[tokio::test]
    async fn server_error_is_retried_until_the_budget_is_spent() {
        let template = ResponseTemplate::new(500).set_body_raw(
            r#"{"status_code":11,"status_message":"Internal error."}"#,
            "application/json",
        );
        // max_retries = 2 → exactly 3 attempts.
        let (_server, client) = search_server(template, 3, |c| c.with_retry(TWO_RETRIES)).await;

        let err = client.search_movies("x", None).await.unwrap_err();
        assert!(
            matches!(
                err,
                TmdbError::Api {
                    status: 500,
                    status_code: Some(11),
                    ..
                }
            ),
            "got {err:?}"
        );
    }

    #[tokio::test]
    async fn client_errors_are_attempted_exactly_once() {
        for status in [400u16, 401, 404, 422] {
            let template = ResponseTemplate::new(status).set_body_raw(
                r#"{"status_code":34,"status_message":"nope"}"#,
                "application/json",
            );
            let (server, client) = search_server(template, 1, |c| c.with_retry(TWO_RETRIES)).await;

            let err = client.search_movies("x", None).await.unwrap_err();
            assert!(
                matches!(&err, TmdbError::Api { status: s, .. } if *s == status),
                "HTTP {status}: got {err:?}"
            );
            server.verify().await;
        }
    }

    /// The timeout covers connecting and sending too, and the attempt count
    /// is read from the server, so it is wide (500ms) for each attempt to
    /// reach the server even under heavy parallel load; the 10s delay keeps
    /// the response from racing it.
    #[tokio::test]
    async fn request_timeout_is_retried_until_the_budget_is_spent() {
        let template = ResponseTemplate::new(200)
            .set_body_raw(EMPTY_SEARCH, "application/json")
            .set_delay(Duration::from_secs(10));
        let (_server, client) = search_server(template, 3, |c| {
            c.with_retry(TWO_RETRIES)
                .with_request_timeout(Duration::from_millis(500))
        })
        .await;

        match client.search_movies("x", None).await.unwrap_err() {
            TmdbError::Http(e) => assert!(e.is_timeout(), "expected a timeout, got {e:?}"),
            other => panic!("expected Http timeout, got {other:?}"),
        }
    }

    /// Connect failures never reach a server, so the attempt count is
    /// observed through the backoff: two retries at `base_delay = max_delay =
    /// 200ms` wait at least 100ms each (equal jitter keeps half the delay).
    #[tokio::test]
    async fn connect_error_is_retried_with_backoff() {
        let policy = RetryPolicy::new(2, Duration::from_millis(200), Duration::from_millis(200));
        let config = TmdbConfig::new_with_base_url("t", "http://127.0.0.1:1").with_retry(policy);
        let client = TmdbClient::new(config).unwrap();

        let started = std::time::Instant::now();
        let err = client.search_movies("x", None).await.unwrap_err();
        let elapsed = started.elapsed();

        match err {
            TmdbError::Http(e) => assert!(e.is_connect(), "expected a connect error, got {e:?}"),
            other => panic!("expected Http connect error, got {other:?}"),
        }
        assert!(
            elapsed >= Duration::from_millis(200),
            "two retries should back off >= 200ms in total, took {elapsed:?}"
        );
    }

    /// `Retry-After` overrides the (tiny) exponential backoff.
    #[tokio::test]
    async fn retry_after_header_sets_the_retry_delay() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/3/search/movie"))
            .respond_with(
                ResponseTemplate::new(429)
                    .insert_header("Retry-After", "1")
                    .set_body_raw(
                        r#"{"status_code":25,"status_message":"limit"}"#,
                        "application/json",
                    ),
            )
            .up_to_n_times(1)
            .with_priority(1)
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/3/search/movie"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(EMPTY_SEARCH, "application/json"))
            .with_priority(2)
            .expect(1)
            .mount(&server)
            .await;
        let config = TmdbConfig::new_with_base_url("t", server.uri()).with_retry(TWO_RETRIES);
        let client = TmdbClient::new(config).unwrap();

        let started = std::time::Instant::now();
        let page = client.search_movies("x", None).await.unwrap();
        let elapsed = started.elapsed();

        assert_eq!(page.total_results, 0);
        assert!(
            elapsed >= Duration::from_secs(1),
            "Retry-After: 1 should delay the retry >= 1s, took {elapsed:?}"
        );
    }

    /// Mount one `first` response on `route`, then `200 EMPTY_SEARCH` for
    /// every later request, each hit exactly once, and build a client with
    /// [`TWO_RETRIES`].
    async fn fail_once_then_succeed(
        route: &str,
        first: ResponseTemplate,
    ) -> (MockServer, TmdbClient) {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path(route))
            .respond_with(first)
            .up_to_n_times(1)
            .with_priority(1)
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path(route))
            .respond_with(ResponseTemplate::new(200).set_body_raw(EMPTY_SEARCH, "application/json"))
            .with_priority(2)
            .expect(1)
            .mount(&server)
            .await;
        let config = TmdbConfig::new_with_base_url("t", server.uri()).with_retry(TWO_RETRIES);
        (server, TmdbClient::new(config).unwrap())
    }

    /// The discover-movie builder runs through the retry seam: a 429 is
    /// retried, and the second attempt's page is returned.
    #[tokio::test]
    async fn discover_movies_retries_429_then_succeeds() {
        let first = ResponseTemplate::new(429)
            .insert_header("Retry-After", "0")
            .set_body_raw(
                r#"{"status_code":25,"status_message":"limit"}"#,
                "application/json",
            );
        let (_server, client) = fail_once_then_succeed("/3/discover/movie", first).await;

        let page = client
            .discover_movies()
            .execute()
            .await
            .expect("recovers after one 429");
        assert_eq!(page.page, 1);
        assert!(page.results.is_empty());
    }

    /// The discover-TV builder runs through the retry seam: a 5xx is retried,
    /// and the second attempt's page is returned.
    #[tokio::test]
    async fn discover_tv_retries_5xx_then_succeeds() {
        let first = ResponseTemplate::new(503).set_body_raw(
            r#"{"status_code":11,"status_message":"err"}"#,
            "application/json",
        );
        let (_server, client) = fail_once_then_succeed("/3/discover/tv", first).await;

        let page = client
            .discover_tv()
            .execute()
            .await
            .expect("recovers after one 503");
        assert_eq!(page.page, 1);
        assert!(page.results.is_empty());
    }

    /// A discover 4xx other than 429 is attempted exactly once, and its error
    /// carries TMDB's decoded `status_code` and `status_message`.
    #[tokio::test]
    async fn discover_client_error_is_attempted_once_and_decoded() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/3/discover/movie"))
            .respond_with(ResponseTemplate::new(401).set_body_raw(
                r#"{"status_code":7,"status_message":"Invalid API key"}"#,
                "application/json",
            ))
            .expect(1)
            .mount(&server)
            .await;
        let config = TmdbConfig::new_with_base_url("t", server.uri()).with_retry(TWO_RETRIES);
        let client = TmdbClient::new(config).unwrap();

        let err = client.discover_movies().execute().await.unwrap_err();
        match err {
            TmdbError::Api {
                status,
                status_code,
                message,
                ..
            } => {
                assert_eq!(status, 401);
                assert_eq!(status_code, Some(7));
                assert_eq!(message, "Invalid API key");
            }
            other => panic!("expected Api error, got {other:?}"),
        }
    }
}

// ── AniList ───────────────────────────────────────────────────────────────────

#[cfg(feature = "anilist")]
mod anilist {
    use cameo::{
        ProviderError,
        providers::anilist::{AniListClient, AniListConfig, AniListError},
    };

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
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(empty_page()))
            .with_priority(2)
            .expect(1)
            .mount(&server)
            .await;

        let config = AniListConfig::new_with_base_url(server.uri()).with_retry(FAST_RETRY);
        let client = AniListClient::new(config).unwrap();

        // One 429, then one success: exactly two attempts (verified on drop).
        let page = client
            .search_tv_shows("x", None)
            .await
            .expect("recovers after one 429");
        assert_eq!(page.page, 1);
        assert_eq!(page.total_pages, 1);
        assert_eq!(page.total_results, 0);
        assert!(page.results.is_empty());
    }

    /// Mount a single response for every POST, asserting it is hit exactly
    /// `expected` times, and build a client with a 2-retry fast policy.
    async fn client_expecting(
        template: ResponseTemplate,
        expected: u64,
    ) -> (MockServer, AniListClient) {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(template)
            .expect(expected)
            .mount(&server)
            .await;
        let policy = RetryPolicy::new(2, Duration::from_millis(10), Duration::from_millis(20));
        let config = AniListConfig::new_with_base_url(server.uri()).with_retry(policy);
        (server, AniListClient::new(config).unwrap())
    }

    fn assert_graphql_500(err: AniListError) {
        match &err {
            AniListError::GraphQL { errors, .. } => assert_eq!(errors[0].status, Some(500)),
            other => panic!("expected GraphQL error, got {other:?}"),
        }
        assert!(
            matches!(
                ProviderError::from(err),
                ProviderError::Api { status: 500, .. }
            ),
            "GraphQL 5xx must map to ProviderError::Api"
        );
    }

    #[tokio::test]
    async fn details_null_root_with_5xx_is_retried_then_surfaced() {
        let body = serde_json::json!({
            "data": { "Media": null },
            "errors": [{ "message": "Internal", "status": 500 }]
        });
        // max_retries = 2 → exactly 3 attempts.
        let (_server, client) =
            client_expecting(ResponseTemplate::new(200).set_body_json(body), 3).await;

        let err = client.movie_details(1).await.unwrap_err();
        assert_graphql_500(err);
    }

    #[tokio::test]
    async fn details_null_root_with_5xx_recovers_on_retry() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": { "Media": null },
                "errors": [{ "message": "Internal", "status": 500 }]
            })))
            .up_to_n_times(1)
            .with_priority(1)
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(
                include_str!("../fixtures/anilist_media_details_response.json"),
                "application/json",
            ))
            .with_priority(2)
            .expect(1)
            .mount(&server)
            .await;
        let config = AniListConfig::new_with_base_url(server.uri()).with_retry(FAST_RETRY);
        let client = AniListClient::new(config).unwrap();

        let details = client.movie_details(1535).await.unwrap();
        assert_eq!(details.movie.provider_id, "anilist:1535");
    }

    #[tokio::test]
    async fn list_null_page_with_5xx_is_retried_then_surfaced() {
        let body = serde_json::json!({
            "data": { "Page": null },
            "errors": [{ "message": "Internal", "status": 500 }]
        });
        let (_server, client) =
            client_expecting(ResponseTemplate::new(200).set_body_json(body), 3).await;

        let err = client.popular_movies(None).await.unwrap_err();
        assert_graphql_500(err);
    }

    #[tokio::test]
    async fn staff_list_null_page_with_5xx_is_retried_then_surfaced() {
        let body = serde_json::json!({
            "data": { "Page": null },
            "errors": [{ "message": "Internal", "status": 500 }]
        });
        let (_server, client) =
            client_expecting(ResponseTemplate::new(200).set_body_json(body), 3).await;

        let err = client.search_people("x", None).await.unwrap_err();
        assert_graphql_500(err);
    }

    #[tokio::test]
    async fn http_5xx_with_statusless_errors_inherits_http_status() {
        let body = serde_json::json!({
            "data": { "Staff": null },
            "errors": [{ "message": "Internal" }]
        });
        let (_server, client) =
            client_expecting(ResponseTemplate::new(500).set_body_json(body), 3).await;

        let err = client.person_details(1).await.unwrap_err();
        assert_graphql_500(err);
    }

    #[tokio::test]
    async fn null_root_with_404_is_not_found_without_retry() {
        let body = serde_json::json!({
            "data": { "Media": null },
            "errors": [{ "message": "Not Found.", "status": 404 }]
        });
        let (_server, client) =
            client_expecting(ResponseTemplate::new(404).set_body_json(body), 1).await;

        let err = client.tv_show_details(1).await.unwrap_err();
        assert!(matches!(err, AniListError::NotFound), "got {err:?}");
    }

    #[tokio::test]
    async fn null_root_without_errors_keeps_not_found_and_no_data() {
        let (_server, client) = client_expecting(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({ "data": { "Media": null } })),
            1,
        )
        .await;
        let err = client.movie_details(1).await.unwrap_err();
        assert!(matches!(err, AniListError::NotFound), "got {err:?}");

        let (_server, client) = client_expecting(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({ "data": { "Page": null } })),
            1,
        )
        .await;
        let err = client
            .trending_tv_shows(cameo::TimeWindow::Day, None)
            .await
            .unwrap_err();
        assert!(matches!(err, AniListError::NoData), "got {err:?}");
    }

    #[tokio::test]
    async fn http_404_with_statusless_errors_is_not_found() {
        let body = serde_json::json!({
            "data": { "Media": null },
            "errors": [{ "message": "Not Found." }]
        });
        let (_server, client) =
            client_expecting(ResponseTemplate::new(404).set_body_json(body), 1).await;

        let err = client.movie_details(1).await.unwrap_err();
        assert!(matches!(err, AniListError::NotFound), "got {err:?}");
    }

    fn assert_status_503(err: AniListError) {
        assert!(
            matches!(err, AniListError::Status { status: 503, .. }),
            "expected Status 503, got {err:?}"
        );
    }

    #[tokio::test]
    async fn http_503_null_root_without_errors_is_retried_then_status() {
        let body = serde_json::json!({ "data": { "Media": null } });
        let (_server, client) =
            client_expecting(ResponseTemplate::new(503).set_body_json(body), 3).await;

        let err = client.movie_details(1).await.unwrap_err();
        assert_status_503(err);
    }

    #[tokio::test]
    async fn http_503_null_page_with_empty_errors_is_retried_then_status() {
        let body = serde_json::json!({ "data": { "Page": null }, "errors": [] });
        let (_server, client) =
            client_expecting(ResponseTemplate::new(503).set_body_json(body), 3).await;

        let err = client.popular_tv_shows(None).await.unwrap_err();
        assert_status_503(err);
    }

    #[tokio::test]
    async fn null_root_with_non_error_status_errors_is_graphql_not_not_found() {
        // The server reported errors, so a null root is not claimed missing.
        let body = serde_json::json!({
            "data": { "Media": null },
            "errors": [{ "message": "Some field failed to resolve.", "status": 200 }]
        });
        let (_server, client) =
            client_expecting(ResponseTemplate::new(200).set_body_json(body), 1).await;

        let err = client.movie_details(1).await.unwrap_err();
        assert!(
            matches!(err, AniListError::GraphQL { .. }),
            "expected GraphQL error, got {err:?}"
        );
    }

    #[tokio::test]
    async fn partial_data_with_non_null_root_and_5xx_errors_is_ok() {
        let mut body: serde_json::Value = serde_json::from_str(include_str!(
            "../fixtures/anilist_media_details_response.json"
        ))
        .unwrap();
        body["errors"] = serde_json::json!([{ "message": "Internal", "status": 500 }]);
        let (_server, client) =
            client_expecting(ResponseTemplate::new(200).set_body_json(body), 1).await;

        let details = client.movie_details(1535).await.unwrap();
        assert_eq!(details.movie.provider_id, "anilist:1535");
    }
}
