use std::{fmt, time::Duration};

use reqwest::header::HeaderMap;

use crate::generated::tmdb as generated;

/// Error type for the TMDB provider.
///
/// Failures fall into three clearly separated categories so callers can react
/// appropriately without string-matching:
///
/// * [`TmdbError::Http`] — a transport failure (connection, DNS, TLS, timeout)
///   where **no** HTTP response was received.
/// * [`TmdbError::Api`] — the server returned an error **status**, carrying the
///   real HTTP status, TMDB's decoded `{status_code, status_message}` body, and
///   any `Retry-After` delay.
/// * [`TmdbError::Deserialization`] — a response body could not be decoded into
///   the expected type.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum TmdbError {
    /// HTTP transport failure (connection, DNS, TLS, or timeout).
    ///
    /// No HTTP response was received — distinct from [`TmdbError::Api`], which
    /// always carries a real HTTP status.
    ///
    /// The [`TransportError`] never holds the request URL: a v3 API key
    /// travels in the URL's `api_key` query parameter and would otherwise leak
    /// through `Display`/`Debug`.
    #[error("HTTP transport error: {0}")]
    Http(#[source] TransportError),

    /// The TMDB API returned an error status.
    ///
    /// `status` is the real HTTP response code (e.g. 401, 404, 429) so callers
    /// can distinguish authentication failures from missing resources or rate
    /// limiting by matching on it. `status_code` and `message` are decoded from
    /// TMDB's `{status_code, status_message}` error body when present, and
    /// `retry_after` carries the `Retry-After` delay for 429/503 responses so a
    /// retry layer can honor it.
    #[error("TMDB API error (HTTP {status}): {message}")]
    Api {
        /// HTTP response status code.
        status: u16,
        /// TMDB's own numeric status code (from the error body), if present.
        status_code: Option<i64>,
        /// Human-readable message (TMDB's `status_message`, or a fallback).
        message: String,
        /// The `Retry-After` delay, if the response supplied one.
        retry_after: Option<Duration>,
    },

    /// A response body could not be deserialized into the expected type.
    #[error("deserialization error: {0}")]
    Deserialization(#[from] serde_json::Error),

    /// Invalid configuration (rejected at construction time).
    #[error("invalid configuration: {0}")]
    InvalidConfig(String),

    /// TMDB cannot look titles up by ids from this external source (see
    /// [`TmdbClient::find_by_external_id`](super::TmdbClient::find_by_external_id)).
    #[error("TMDB cannot find titles by {0} id")]
    UnsupportedSource(crate::unified::external_ids::ExternalSource),
}

/// A transport failure: no usable HTTP response was received.
///
/// Carried by [`TmdbError::Http`]. It classifies the failure the way a retry
/// layer needs ([`is_connect`](Self::is_connect),
/// [`is_timeout`](Self::is_timeout)) and describes it in its `Display` output.
/// It never holds or prints the request URL's `api_key` query value: the
/// description has every `api_key` value replaced with `<redacted>`.
#[derive(Debug, Clone)]
pub struct TransportError {
    kind: TransportKind,
    message: String,
}

/// The class of a [`TransportError`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TransportKind {
    /// Connection-level: DNS, refused or reset connection, TLS handshake.
    Connect,
    /// A connect or total-request timeout elapsed.
    Timeout,
    /// Anything else: malformed HTTP, redirect exhaustion, a body cut short.
    Other,
}

impl TransportError {
    fn new(kind: TransportKind, message: &str) -> Self {
        Self {
            kind,
            message: redact_api_key(message),
        }
    }

    /// Whether the connection could not be established (DNS, refused or reset
    /// connection, TLS handshake). Transient: worth retrying.
    pub fn is_connect(&self) -> bool {
        self.kind == TransportKind::Connect
    }

    /// Whether a connect or total-request timeout elapsed. Transient: worth
    /// retrying.
    pub fn is_timeout(&self) -> bool {
        self.kind == TransportKind::Timeout
    }
}

impl fmt::Display for TransportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for TransportError {}

/// Wrap a transport error, stripping its request URL so a v3 `api_key` query
/// parameter can never surface through `Display` or `Debug`.
impl From<reqwest::Error> for TmdbError {
    fn from(err: reqwest::Error) -> Self {
        let kind = if err.is_timeout() {
            TransportKind::Timeout
        } else if err.is_connect() {
            TransportKind::Connect
        } else {
            TransportKind::Other
        };
        let err = err.without_url();
        TmdbError::Http(TransportError::new(kind, &error_chain(&err)))
    }
}

/// TMDB's structured error body, e.g. `{"status_code": 7, "status_message": "..."}`.
#[derive(Debug, serde::Deserialize)]
struct TmdbErrorBody {
    status_code: Option<i64>,
    status_message: Option<String>,
}

impl TmdbError {
    /// Whether this failure is transient and worth retrying: a 429/5xx API
    /// status, or a connect/timeout transport error.
    pub(crate) fn is_retryable(&self) -> bool {
        match self {
            TmdbError::Api { status, .. } => *status == 429 || (500..=599).contains(status),
            TmdbError::Http(e) => e.is_timeout() || e.is_connect(),
            _ => false,
        }
    }

    /// The server-supplied `Retry-After` delay, if any.
    pub(crate) fn retry_after(&self) -> Option<Duration> {
        match self {
            TmdbError::Api { retry_after, .. } => *retry_after,
            _ => None,
        }
    }

    /// Map a generated-client error into a structured [`TmdbError`], decoding
    /// the TMDB error body and `Retry-After` header for error responses.
    ///
    /// spargen retains the (capped) body of every error response, so the
    /// mapping is synchronous. Its own `Display` is not used for transport
    /// failures: the underlying `reqwest::Error` prints the request URL, which
    /// carries a v3 `api_key`, so the description is redacted first.
    pub(crate) fn from_generated<E: fmt::Debug>(err: generated::Error<E>) -> Self {
        use generated::Error as G;
        match err {
            G::Transport(e) => Self::transport(TransportKind::Connect, &e),
            G::Timeout(kind) => TmdbError::Http(TransportError::new(
                TransportKind::Timeout,
                &format!("{kind:?} timeout elapsed"),
            )),
            G::Protocol(e) => Self::transport(TransportKind::Other, &e),
            G::Redirect(e) => Self::transport(TransportKind::Other, &e),
            G::InterruptedBody(e) => Self::transport(TransportKind::Other, &e),

            // We failed to even build the request (e.g. a missing credential).
            G::RequestConstruction(e) => TmdbError::Api {
                status: 0,
                status_code: None,
                message: redact_api_key(&format!("invalid request: {}", error_chain(&e))),
                retry_after: None,
            },
            G::PaginationCycle { .. } => TmdbError::Api {
                status: 0,
                status_code: None,
                message: "invalid request: pagination cycle".to_string(),
                retry_after: None,
            },

            // A declared error response with a typed body.
            G::Api(rv) => {
                let status = rv.status().as_u16();
                let retry_after = parse_retry_after(rv.headers());
                TmdbError::Api {
                    status,
                    status_code: None,
                    message: format!("{:?}", rv.into_inner()),
                    retry_after,
                }
            }

            // An undeclared error status — the common TMDB path. Decode the
            // `{status_code, status_message}` body ourselves.
            G::UnexpectedStatus {
                status,
                headers,
                body,
            } => Self::api(status.as_u16(), &headers, &body),

            // A success body that did not match the schema is a decode
            // failure; an error status whose body did not match is still an
            // API error.
            G::Decode {
                status,
                headers,
                path,
                body,
                ..
            } => {
                if status.is_success() {
                    TmdbError::Deserialization(<serde_json::Error as serde::de::Error>::custom(
                        path,
                    ))
                } else {
                    Self::api(status.as_u16(), &headers, &body)
                }
            }
        }
    }

    /// A [`TmdbError::Http`] of `kind` describing `err` (redacted).
    fn transport(kind: TransportKind, err: &(dyn std::error::Error + 'static)) -> Self {
        TmdbError::Http(TransportError::new(kind, &error_chain(err)))
    }

    /// A [`TmdbError::Api`] for an error `status`, decoding TMDB's error body.
    fn api(status: u16, headers: &HeaderMap, body: &[u8]) -> Self {
        let retry_after = parse_retry_after(headers);
        let (status_code, message) = decode_tmdb_error_body(&String::from_utf8_lossy(body), status);
        TmdbError::Api {
            status,
            status_code,
            message,
            retry_after,
        }
    }
}

/// `err` and each of its sources, joined with `": "`, skipping a source whose
/// text its parent already ends with.
///
/// A `reqwest::Error` in the chain prints its request URL; every such URL is
/// written without its query string (where a v3 `api_key` travels).
fn error_chain(err: &(dyn std::error::Error + 'static)) -> String {
    let mut out = err.to_string();
    let mut urls = Vec::new();
    let mut current = Some(err);
    while let Some(e) = current {
        if let Some(url) = e
            .downcast_ref::<reqwest::Error>()
            .and_then(reqwest::Error::url)
        {
            urls.push(url.clone());
        }
        if !std::ptr::addr_eq(e, err) {
            let text = e.to_string();
            if !out.ends_with(&text) {
                out.push_str(": ");
                out.push_str(&text);
            }
        }
        current = e.source();
    }
    for url in urls {
        let mut bare = url.clone();
        bare.set_query(None);
        out = out.replace(url.as_str(), bare.as_str());
    }
    out
}

/// Replace the value of every `api_key=` occurrence in `text` with
/// `<redacted>`. The value ends at the next `&`, `#`, `)`, quote, or
/// whitespace character.
fn redact_api_key(text: &str) -> String {
    const NEEDLE: &str = "api_key=";
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find(NEEDLE) {
        let value_start = at + NEEDLE.len();
        out.push_str(&rest[..value_start]);
        let value = &rest[value_start..];
        let value_len = value
            .find(|c: char| matches!(c, '&' | '#' | ')' | '"' | '\'') || c.is_whitespace())
            .unwrap_or(value.len());
        if value_len > 0 {
            out.push_str("<redacted>");
        }
        rest = &value[value_len..];
    }
    out.push_str(rest);
    out
}

/// Parse a `Retry-After` header (integer seconds) into a [`Duration`].
///
/// TMDB and AniList both send an integer number of seconds. HTTP-date form is
/// not parsed (returns `None`).
fn parse_retry_after(headers: &HeaderMap) -> Option<Duration> {
    let raw = headers.get(reqwest::header::RETRY_AFTER)?.to_str().ok()?;
    raw.trim().parse::<u64>().ok().map(Duration::from_secs)
}

/// Decode TMDB's `{status_code, status_message}` error body, falling back to the
/// raw body (or a generic message) when it is absent or malformed.
fn decode_tmdb_error_body(body: &str, status: u16) -> (Option<i64>, String) {
    match serde_json::from_str::<TmdbErrorBody>(body) {
        Ok(parsed) => {
            let message = parsed
                .status_message
                .filter(|m| !m.is_empty())
                .unwrap_or_else(|| format!("HTTP {status}"));
            (parsed.status_code, message)
        }
        Err(_) => {
            let trimmed = body.trim();
            let message = if trimmed.is_empty() {
                format!("HTTP {status}")
            } else {
                trimmed.to_string()
            };
            (None, message)
        }
    }
}

#[cfg(test)]
mod tests {
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
        assert!(matches!(err, TmdbError::Deserialization(_)), "got {err:?}");
        assert!(err.to_string().contains("expected i64"), "{err}");
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
}
