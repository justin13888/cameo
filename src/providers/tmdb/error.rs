use std::{fmt, time::Duration};

use reqwest::header::HeaderMap;

use crate::{
    core::error::{DecodeError, ProviderError, TransportError, TransportKind},
    generated::tmdb as generated,
    providers::{
        http::{is_retryable_status, parse_retry_after},
        retry::Retryable,
    },
};

/// Error type for the TMDB provider.
///
/// Request failures fall into four clearly separated categories so callers can
/// react appropriately without string-matching:
///
/// * [`TmdbError::Request`] — the request was never sent, or the client stopped
///   on its own: it could not be built (e.g. a parameter TMDB cannot carry) or
///   a pagination `next` link revisited an earlier page. No HTTP response is
///   involved.
/// * [`TmdbError::Http`] — a transport failure (connection, DNS, TLS, timeout)
///   where **no** HTTP response was received.
/// * [`TmdbError::Api`] — the server returned an error **status**, carrying the
///   real HTTP status, TMDB's decoded `{status_code, status_message}` body, and
///   any `Retry-After` delay. `status` is always a real HTTP status code.
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
    /// through `Display`/`Debug`. A failure raised inside the request pipeline
    /// has no [`source`](std::error::Error::source), because its original
    /// cannot be separated from that URL.
    #[error("HTTP transport error: {0}")]
    Http(#[source] TransportError),

    /// The TMDB API returned an error status.
    ///
    /// Only ever constructed from a received HTTP response: failures that never
    /// reached the server are [`TmdbError::Request`] or [`TmdbError::Http`].
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
    ///
    /// The [`DecodeError`] carries the decoder's message, which is all the
    /// request pipeline reports, so it has no
    /// [`source`](std::error::Error::source).
    #[error("deserialization error: {0}")]
    Deserialization(#[source] DecodeError),

    /// The request failed without any HTTP response being involved.
    ///
    /// [`RequestErrorKind::InvalidRequest`]: the request could not be built
    /// (e.g. a date outside the RFC 3339 range, or a missing credential), so
    /// nothing was sent. [`RequestErrorKind::PaginationCycle`]: the client
    /// stopped following TMDB's pagination because a `next` link revisited an
    /// earlier page. Neither is retryable.
    ///
    /// `message` never carries a v3 `api_key` value.
    #[error("{kind}: {message}")]
    Request {
        /// What kind of request failure this is.
        kind: RequestErrorKind,
        /// Human-readable description of the failure.
        message: String,
    },

    /// A response reported a page number or total that does not fit the
    /// `u32` a [`Page`](crate::core::pagination::Page) carries (negative, or
    /// above `u32::MAX`).
    ///
    /// The value is reported rather than wrapped or clamped, so a malformed
    /// page is never passed off as a real one. Not retryable.
    #[error("TMDB reported an out-of-range `{field}`: {value}")]
    PageOutOfRange {
        /// The pagination field: `page`, `total_pages`, or `total_results`.
        field: &'static str,
        /// The value TMDB reported.
        value: i64,
    },

    /// Invalid configuration (rejected at construction time).
    #[error("invalid configuration: {0}")]
    InvalidConfig(String),

    /// TMDB cannot look titles up by ids from this external source (see
    /// [`TmdbClient::find_by_external_id`](super::TmdbClient::find_by_external_id)).
    #[error("TMDB cannot find titles by {0} id")]
    UnsupportedSource(crate::unified::external_ids::ExternalSource),
}

/// The kind of a [`TmdbError::Request`] failure.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RequestErrorKind {
    /// The request could not be built, so nothing was sent: the caller's input
    /// or configuration cannot be expressed as a TMDB request.
    InvalidRequest,
    /// A pagination `next` link revisited a page already fetched, so the
    /// client stopped rather than loop forever.
    PaginationCycle,
}

impl fmt::Display for RequestErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            RequestErrorKind::InvalidRequest => "invalid request",
            RequestErrorKind::PaginationCycle => "pagination cycle",
        })
    }
}

/// TMDB's structured error body, e.g. `{"status_code": 7, "status_message": "..."}`.
#[derive(Debug, serde::Deserialize)]
struct TmdbErrorBody {
    status_code: Option<i64>,
    status_message: Option<String>,
}

impl Retryable for TmdbError {
    /// Whether this failure is transient and worth retrying: a 429/5xx API
    /// status, or a connect/timeout transport error.
    fn is_retryable(&self) -> bool {
        match self {
            TmdbError::Api { status, .. } => is_retryable_status(*status),
            TmdbError::Http(e) => e.is_timeout() || e.is_connect(),
            _ => false,
        }
    }

    /// The server-supplied `Retry-After` delay, if any.
    fn retry_after(&self) -> Option<Duration> {
        match self {
            TmdbError::Api { retry_after, .. } => *retry_after,
            _ => None,
        }
    }
}

impl From<TmdbError> for ProviderError {
    fn from(err: TmdbError) -> Self {
        match err {
            // The `TransportError` description is already redacted: it never
            // carries the request URL's `api_key`.
            TmdbError::Http(e) => ProviderError::Transport(e.to_string()),
            TmdbError::Deserialization(e) => ProviderError::Deserialization(e.to_string()),
            e @ TmdbError::PageOutOfRange { .. } => ProviderError::Deserialization(e.to_string()),
            TmdbError::InvalidConfig(s) => ProviderError::InvalidInput(s),
            TmdbError::UnsupportedSource(_) => ProviderError::Unsupported,
            TmdbError::Request { kind, message } => match kind {
                RequestErrorKind::InvalidRequest => ProviderError::InvalidInput(message),
                RequestErrorKind::PaginationCycle => {
                    ProviderError::Other(format!("{kind}: {message}"))
                }
            },
            TmdbError::Api {
                status,
                message,
                retry_after,
                ..
            } => ProviderError::from_http_status(status, message, retry_after),
        }
    }
}

impl TmdbError {
    /// Wrap a `reqwest` error as [`TmdbError::Http`], stripping its request
    /// URL so a v3 `api_key` query parameter can never surface through
    /// `Display`, `Debug`, or the kept [`source`](std::error::Error::source).
    pub(crate) fn from_reqwest(err: reqwest::Error) -> Self {
        TmdbError::Http(TransportError::from_reqwest(err, |e| {
            redact_api_key(&error_chain(e))
        }))
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
                format!("{kind:?} timeout elapsed"),
            )),
            G::Protocol(e) => Self::transport(TransportKind::Other, &e),
            G::Redirect(e) => Self::transport(TransportKind::Other, &e),
            G::InterruptedBody(e) => Self::transport(TransportKind::Other, &e),

            // reqwest failed inside the send (e.g. the connection closed
            // before any response): no response was received, so this is a
            // transport failure, not an API error.
            G::RequestConstruction(e) if is_send_failure(&e) => {
                Self::transport(TransportKind::Other, &e)
            }
            // We failed to even build the request (e.g. a missing credential).
            G::RequestConstruction(e) => TmdbError::Request {
                kind: RequestErrorKind::InvalidRequest,
                message: redact_api_key(&error_chain(&e)),
            },
            // The revisited URL is left out: it may carry a v3 `api_key`.
            G::PaginationCycle { .. } => TmdbError::Request {
                kind: RequestErrorKind::PaginationCycle,
                message: "a `next` link revisited an earlier page".to_string(),
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
                    TmdbError::Deserialization(DecodeError::new(path))
                } else {
                    Self::api(status.as_u16(), &headers, &body)
                }
            }
        }
    }

    /// A [`TmdbError::Http`] of `kind` describing `err` (redacted).
    fn transport(kind: TransportKind, err: &(dyn std::error::Error + 'static)) -> Self {
        TmdbError::Http(TransportError::new(kind, redact_api_key(&error_chain(err))))
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

/// Whether a request-construction error is really a failure inside the send:
/// a request-kind `reqwest::Error` (not builder-kind, which reqwest raises
/// before sending anything), e.g. a connection closed before any response.
fn is_send_failure(err: &generated::RequestError) -> bool {
    matches!(err, generated::RequestError::Other(_))
        && std::error::Error::source(err)
            .and_then(|e| e.downcast_ref::<reqwest::Error>())
            .is_some_and(|e| e.is_request() && !e.is_builder())
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
mod tests;
