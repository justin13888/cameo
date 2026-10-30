//! TMDB credentials must never surface through error or `Debug` output
//! (issue #61). A v3 API key travels in the request URL, so transport errors
//! must have their URL stripped; config/client `Debug` must redact secrets.

use cameo::{
    RetryPolicy,
    core::error::ProviderError,
    providers::tmdb::{TmdbAuth, TmdbClient, TmdbConfig, TmdbError},
    unified::CameoClient,
};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path},
};

const SECRET: &str = "SECRETKEY123";

/// A port that is (almost certainly) closed, so requests fail at connect time.
const CLOSED: &str = "http://127.0.0.1:1";

/// Both auth schemes, each built around the distinctive secret.
fn configs(base_url: &str) -> [TmdbConfig; 2] {
    [TmdbConfig::new_v3(SECRET), TmdbConfig::new(SECRET)].map(|c| {
        c.with_base_url(base_url)
            .with_retry(RetryPolicy::disabled())
    })
}

/// Every rendering of an error a caller might log: `Display`, alternate
/// `Display`, `Debug`, pretty `Debug`, and each `source()` in the chain.
fn renderings(err: &(dyn std::error::Error + 'static)) -> Vec<String> {
    let mut out = vec![
        format!("{err}"),
        format!("{err:#}"),
        format!("{err:?}"),
        format!("{err:#?}"),
    ];
    let mut src = err.source();
    while let Some(s) = src {
        out.push(format!("{s}"));
        out.push(format!("{s:?}"));
        src = s.source();
    }
    out
}

fn assert_no_secret(err: &(dyn std::error::Error + 'static)) {
    for rendered in renderings(err) {
        assert!(
            !rendered.contains(SECRET),
            "credential leaked in error output: {rendered}"
        );
    }
}

#[tokio::test]
async fn transport_error_omits_v3_key_and_url() {
    let client = TmdbClient::new(configs(CLOSED)[0].clone()).unwrap();

    let err = client.movie_details(550).await.unwrap_err();
    assert!(matches!(err, TmdbError::Http(_)), "got {err:?}");
    assert_no_secret(&err);
    assert!(
        !format!("{err:?}").contains("api_key"),
        "request URL should be stripped: {err:?}"
    );

    let provider_err = ProviderError::from(err);
    assert!(
        matches!(provider_err, ProviderError::Transport(_)),
        "got {provider_err:?}"
    );
    assert_no_secret(&provider_err);
}

#[tokio::test]
async fn transport_error_is_still_retryable_classified() {
    // Stripping the URL must not change transport classification: a connect
    // failure is still reported as such.
    let client = TmdbClient::new(configs(CLOSED)[0].clone()).unwrap();
    let err = client.movie_details(550).await.unwrap_err();
    match err {
        TmdbError::Http(e) => {
            assert!(e.is_connect(), "expected connect error, got {e:?}");
            assert!(e.url().is_none(), "URL should be stripped");
        }
        other => panic!("expected Http, got {other:?}"),
    }
}

#[tokio::test]
async fn discover_builder_errors_omit_v3_key() {
    // The discover builders map errors through the synchronous `From` impl,
    // a separate path from the main request seam.
    let client = TmdbClient::new(configs(CLOSED)[0].clone()).unwrap();
    let err = client.discover_movies().execute().await.unwrap_err();
    assert_no_secret(&err);

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/3/discover/movie"))
        .respond_with(ResponseTemplate::new(401).set_body_raw(
            r#"{"status_code":7,"status_message":"Invalid API key"}"#,
            "application/json",
        ))
        .mount(&server)
        .await;
    let client = TmdbClient::new(configs(&server.uri())[0].clone()).unwrap();
    let err = client.discover_movies().execute().await.unwrap_err();
    assert!(
        matches!(err, TmdbError::Api { status: 401, .. }),
        "got {err:?}"
    );
    assert_no_secret(&err);
    assert_no_secret(&ProviderError::from(err));
}

#[tokio::test]
async fn facade_error_omits_v3_key() {
    let client = CameoClient::builder()
        .with_tmdb(configs(CLOSED)[0].clone())
        .build()
        .unwrap();

    let err = client.popular_movies(None).await.unwrap_err();
    assert_no_secret(&err);
}

#[test]
fn auth_debug_redacts_secret() {
    for auth in [
        TmdbAuth::V3ApiKey(SECRET.to_string()),
        TmdbAuth::V4Bearer(SECRET.to_string()),
    ] {
        for rendered in [format!("{auth:?}"), format!("{auth:#?}")] {
            assert!(!rendered.contains(SECRET), "leaked: {rendered}");
            assert!(rendered.contains("<redacted>"), "{rendered}");
        }
        // The credential is still readable on purpose.
        assert_eq!(auth.credential(), SECRET);
    }
    assert_eq!(
        format!("{:?}", TmdbAuth::V3ApiKey(SECRET.to_string())),
        "V3ApiKey(<redacted>)"
    );
    assert_eq!(
        format!("{:?}", TmdbAuth::V4Bearer(SECRET.to_string())),
        "V4Bearer(<redacted>)"
    );
}

#[test]
fn config_debug_redacts_secret_but_keeps_other_fields() {
    for config in configs("http://mock.invalid") {
        let config = config.with_language("en-US");
        for rendered in [format!("{config:?}"), format!("{config:#?}")] {
            assert!(!rendered.contains(SECRET), "leaked: {rendered}");
            assert!(rendered.contains("<redacted>"), "{rendered}");
            assert!(rendered.contains("http://mock.invalid"), "{rendered}");
            assert!(rendered.contains("en-US"), "{rendered}");
        }
    }
}

#[test]
fn client_debug_redacts_secret() {
    for config in configs("http://mock.invalid") {
        let client = TmdbClient::new(config).unwrap();
        for rendered in [format!("{client:?}"), format!("{client:#?}")] {
            assert!(!rendered.contains(SECRET), "leaked: {rendered}");
            assert!(rendered.contains("<redacted>"), "{rendered}");
        }
    }
}
