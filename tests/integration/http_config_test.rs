//! Shared HTTP configuration: timeouts and `User-Agent`, for both providers.

use std::time::Duration;

use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{header_regex, method},
};

// ── TMDB ──────────────────────────────────────────────────────────────────────

#[cfg(feature = "tmdb")]
mod tmdb {
    use cameo::providers::tmdb::{TmdbClient, TmdbConfig, TmdbError};
    use wiremock::matchers::path;

    use super::*;

    /// The client sends a `cameo/<version>` User-Agent. The mock only matches
    /// when that header is present, returning 401; an unmatched request would
    /// yield wiremock's default 404, so a 401 proves the UA was sent.
    #[tokio::test]
    async fn sends_cameo_user_agent() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/3/movie/550"))
            .and(header_regex("user-agent", r"^cameo/\d"))
            .respond_with(ResponseTemplate::new(401).set_body_raw(
                r#"{"status_code":7,"status_message":"Invalid API key."}"#,
                "application/json",
            ))
            .mount(&server)
            .await;
        let client = TmdbClient::new(TmdbConfig::new_with_base_url("t", server.uri())).unwrap();

        match client.movie_details(550).await.unwrap_err() {
            TmdbError::Api { status, .. } => assert_eq!(status, 401, "UA header did not match"),
            other => panic!("expected Api 401, got {other:?}"),
        }
    }

    /// A configured request timeout aborts a slow response as a transport error.
    #[tokio::test]
    async fn request_timeout_is_honored() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/3/movie/550"))
            .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_millis(400)))
            .mount(&server)
            .await;
        let config = TmdbConfig::new_with_base_url("t", server.uri())
            .with_request_timeout(Duration::from_millis(50));
        let client = TmdbClient::new(config).unwrap();

        let err = client.movie_details(550).await.unwrap_err();
        assert!(
            matches!(err, TmdbError::Http(_)),
            "expected transport (timeout) error, got {err:?}"
        );
    }
}

// ── AniList ───────────────────────────────────────────────────────────────────

#[cfg(feature = "anilist")]
mod anilist {
    use cameo::providers::anilist::{AniListClient, AniListConfig, AniListError};

    use super::*;

    fn empty_page() -> serde_json::Value {
        serde_json::json!({
            "data": { "Page": {
                "pageInfo": { "total": 0, "currentPage": 1, "lastPage": 1, "hasNextPage": false, "perPage": 20 },
                "media": []
            }}
        })
    }

    /// The client sends a `cameo/<version>` User-Agent. The mock only matches
    /// when the header is present; an unmatched request would fail, so a
    /// successful call proves the UA was sent.
    #[tokio::test]
    async fn sends_cameo_user_agent() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(header_regex("user-agent", r"^cameo/\d"))
            .respond_with(ResponseTemplate::new(200).set_body_json(empty_page()))
            .mount(&server)
            .await;
        let client = AniListClient::new(AniListConfig::new_with_base_url(server.uri())).unwrap();

        let result = client.search_tv_shows("anything", None).await;
        assert!(result.is_ok(), "UA header did not match: {result:?}");
    }

    /// A configured request timeout aborts a slow response as a transport error.
    #[tokio::test]
    async fn request_timeout_is_honored() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(empty_page())
                    .set_delay(Duration::from_millis(400)),
            )
            .mount(&server)
            .await;
        let config = AniListConfig::new_with_base_url(server.uri())
            .with_request_timeout(Duration::from_millis(50));
        let client = AniListClient::new(config).unwrap();

        let err = client.search_tv_shows("anything", None).await.unwrap_err();
        assert!(
            matches!(err, AniListError::Http(_)),
            "expected transport (timeout) error, got {err:?}"
        );
    }
}
