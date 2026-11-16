use super::*;

#[test]
fn redact_api_key_replaces_every_value() {
    assert_eq!(
        redact_api_key(
            "error sending request for url (http://h/3/movie/1?api_key=SECRET&language=en)"
        ),
        "error sending request for url (http://h/3/movie/1?api_key=<redacted>&language=en)"
    );
    assert_eq!(
        redact_api_key("a?api_key=ONE b?x=1&api_key=TWO"),
        "a?api_key=<redacted> b?x=1&api_key=<redacted>"
    );
    assert_eq!(redact_api_key("no key here"), "no key here");
    assert_eq!(redact_api_key("api_key="), "api_key=");
}

#[test]
fn decode_with_error_status_is_api_error() {
    let err = TmdbError::from_generated::<std::convert::Infallible>(generated::Error::Decode {
        status: reqwest::StatusCode::NOT_FOUND,
        headers: HeaderMap::new(),
        path: "expected value".to_string(),
        body: bytes::Bytes::from_static(br#"{"status_code":34,"status_message":"nope"}"#),
        truncated: false,
    });
    assert!(
        matches!(
            err,
            TmdbError::Api {
                status: 404,
                status_code: Some(34),
                ref message,
                ..
            } if message == "nope"
        ),
        "got {err:?}"
    );
}

#[test]
fn decode_with_success_status_is_deserialization_error() {
    let err = TmdbError::from_generated::<std::convert::Infallible>(generated::Error::Decode {
        status: reqwest::StatusCode::OK,
        headers: HeaderMap::new(),
        path: "invalid type: null, expected i64".to_string(),
        body: bytes::Bytes::new(),
        truncated: false,
    });
    match &err {
        TmdbError::Deserialization(e) => {
            assert_eq!(e.to_string(), "invalid type: null, expected i64");
            assert!(std::error::Error::source(e).is_none(), "{e:?}");
        }
        other => panic!("expected Deserialization, got {other:?}"),
    }
    assert!(err.to_string().contains("expected i64"), "{err}");
}

#[test]
fn request_construction_is_invalid_request_not_api() {
    let err = TmdbError::from_generated(
        generated::Error::<std::convert::Infallible>::request_message(
            "bad param api_key=SECRET".to_string(),
        ),
    );
    match &err {
        TmdbError::Request {
            kind: RequestErrorKind::InvalidRequest,
            message,
        } => {
            assert!(message.contains("bad param"), "{message}");
            assert!(!message.contains("SECRET"), "{message}");
        }
        other => panic!("expected Request, got {other:?}"),
    }
    assert!(!err.is_retryable());
    assert!(err.to_string().starts_with("invalid request: "), "{err}");
}

#[test]
fn pagination_cycle_is_request_error_without_url() {
    let url = reqwest::Url::parse("http://h/3/x?page=2&api_key=SECRET").unwrap();
    let err = TmdbError::from_generated(
        generated::Error::<std::convert::Infallible>::PaginationCycle { url },
    );
    assert!(
        matches!(
            err,
            TmdbError::Request {
                kind: RequestErrorKind::PaginationCycle,
                ..
            }
        ),
        "got {err:?}"
    );
    assert!(!err.is_retryable());
    assert!(!err.to_string().contains("SECRET"), "{err}");
}

#[test]
fn timeout_is_retryable_transport_error() {
    let err = TmdbError::from_generated::<std::convert::Infallible>(generated::Error::Timeout(
        generated::TimeoutKind::Total,
    ));
    match &err {
        TmdbError::Http(e) => assert!(e.is_timeout() && !e.is_connect(), "{e:?}"),
        other => panic!("expected Http, got {other:?}"),
    }
    assert!(err.is_retryable());
}

#[test]
fn maps_tmdb_error() {
    let api = |status| TmdbError::Api {
        status,
        status_code: None,
        message: "x".into(),
        retry_after: Some(Duration::from_secs(3)),
    };
    assert!(matches!(
        ProviderError::from(api(404)),
        ProviderError::NotFound
    ));
    assert!(matches!(
        ProviderError::from(api(401)),
        ProviderError::Auth(_)
    ));
    assert!(matches!(
        ProviderError::from(api(429)),
        ProviderError::RateLimited {
            retry_after: Some(_)
        }
    ));
    assert!(matches!(
        ProviderError::from(api(500)),
        ProviderError::Api { status: 500, .. }
    ));
    assert!(matches!(
        ProviderError::from(TmdbError::InvalidConfig("x".into())),
        ProviderError::InvalidInput(_)
    ));
    assert!(matches!(
        ProviderError::from(TmdbError::UnsupportedSource(
            crate::unified::external_ids::ExternalSource::Mal
        )),
        ProviderError::Unsupported
    ));
}

/// Table-driven: every `TmdbError` variant and every status branch maps to
/// the pinned `ProviderError`, carrying its message or `retry_after`.
#[test]
fn maps_every_tmdb_error_branch() {
    use crate::providers::http::test_support::{json_error, reqwest_builder_error};
    let retry = Some(Duration::from_secs(3));
    let api = |status| TmdbError::Api {
        status,
        status_code: Some(7),
        message: "msg".into(),
        retry_after: retry,
    };
    for status in [401, 403] {
        match ProviderError::from(api(status)) {
            ProviderError::Auth(m) => assert_eq!(m, "msg", "status {status}"),
            other => panic!("status {status}: expected Auth, got {other:?}"),
        }
    }
    assert!(matches!(
        ProviderError::from(api(404)),
        ProviderError::NotFound
    ));
    match ProviderError::from(api(429)) {
        ProviderError::RateLimited { retry_after } => assert_eq!(retry_after, retry),
        other => panic!("expected RateLimited, got {other:?}"),
    }
    for status in [400, 418, 500, 503] {
        match ProviderError::from(api(status)) {
            ProviderError::Api { status: s, message } => {
                assert_eq!((s, message.as_str()), (status, "msg"));
            }
            other => panic!("status {status}: expected Api, got {other:?}"),
        }
    }

    let http = TmdbError::from_reqwest(reqwest_builder_error());
    let http_text = http.to_string();
    // The original stays reachable: TmdbError -> TransportError -> reqwest.
    let transport = std::error::Error::source(&http).expect("TransportError source");
    assert!(
        std::error::Error::source(transport)
            .and_then(|e| e.downcast_ref::<reqwest::Error>())
            .is_some(),
        "{http:?}"
    );
    match ProviderError::from(http) {
        ProviderError::Transport(m) => {
            assert!(!m.is_empty());
            assert!(
                http_text.ends_with(&m),
                "Transport carries the transport description: {m:?} vs {http_text:?}"
            );
        }
        other => panic!("expected Transport, got {other:?}"),
    }

    let json = json_error();
    let json_text = json.to_string();
    match ProviderError::from(TmdbError::Deserialization(DecodeError::json(json))) {
        ProviderError::Deserialization(m) => assert_eq!(m, json_text),
        other => panic!("expected Deserialization, got {other:?}"),
    }

    match ProviderError::from(TmdbError::PageOutOfRange {
        field: "total_pages",
        value: -1,
    }) {
        ProviderError::Deserialization(m) => {
            assert_eq!(m, "TMDB reported an out-of-range `total_pages`: -1");
        }
        other => panic!("expected Deserialization, got {other:?}"),
    }

    match ProviderError::from(TmdbError::InvalidConfig("empty token".into())) {
        ProviderError::InvalidInput(m) => assert_eq!(m, "empty token"),
        other => panic!("expected InvalidInput, got {other:?}"),
    }
    assert!(matches!(
        ProviderError::from(TmdbError::UnsupportedSource(
            crate::unified::external_ids::ExternalSource::Mal
        )),
        ProviderError::Unsupported
    ));
    assert!(matches!(
        ProviderError::from(TmdbError::Request {
            kind: RequestErrorKind::InvalidRequest,
            message: "m".into(),
        }),
        ProviderError::InvalidInput(ref m) if m == "m"
    ));
}

#[test]
fn maps_tmdb_request_error_by_kind() {
    let request = |kind| TmdbError::Request {
        kind,
        message: "boom".into(),
    };
    assert!(matches!(
        ProviderError::from(request(RequestErrorKind::InvalidRequest)),
        ProviderError::InvalidInput(ref m) if m == "boom"
    ));
    assert!(matches!(
        ProviderError::from(request(RequestErrorKind::PaginationCycle)),
        ProviderError::Other(ref m) if m == "pagination cycle: boom"
    ));
}

/// A test error with a fixed text and an optional source.
#[derive(Debug)]
struct Chained(&'static str, Option<Box<Chained>>);

impl fmt::Display for Chained {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

impl std::error::Error for Chained {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.1
            .as_deref()
            .map(|e| e as &(dyn std::error::Error + 'static))
    }
}

/// `error_chain` appends each source's text once, skipping a source whose
/// text its parent already ends with.
#[test]
fn error_chain_joins_distinct_sources_once() {
    let err = Chained(
        "outer",
        Some(Box::new(Chained(
            "middle: inner",
            Some(Box::new(Chained("inner", None))),
        ))),
    );
    assert_eq!(error_chain(&err), "outer: middle: inner");
}
