use std::time::Duration;

use progenitor_client::Error as ProgenitorError;
use reqwest::header::HeaderMap;

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
    /// Errors built by this crate (including via `From<reqwest::Error>`) have
    /// the request URL stripped with [`reqwest::Error::without_url`], because
    /// a v3 API key travels in the URL's `api_key` query parameter and would
    /// otherwise leak through `Display`/`Debug`.
    #[error("HTTP transport error: {0}")]
    Http(#[source] reqwest::Error),

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
}

/// Wrap a transport error, stripping its request URL so a v3 `api_key` query
/// parameter can never surface through `Display` or `Debug`.
impl From<reqwest::Error> for TmdbError {
    fn from(err: reqwest::Error) -> Self {
        TmdbError::Http(err.without_url())
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
}

impl TmdbError {
    /// Map a progenitor client error into a structured [`TmdbError`], decoding
    /// the TMDB error body and `Retry-After` header for error responses.
    ///
    /// This is `async` because reading the body of an unexpected (undeclared)
    /// error response requires awaiting the response stream.
    pub(crate) async fn from_progenitor<E>(err: ProgenitorError<E>) -> Self
    where
        E: std::fmt::Debug + Send + Sync + 'static,
    {
        match err {
            // Transport-level failures: no usable HTTP response.
            ProgenitorError::CommunicationError(e)
            | ProgenitorError::ResponseBodyError(e)
            | ProgenitorError::InvalidUpgrade(e) => TmdbError::from(e),

            // A successful response whose body did not match the schema.
            ProgenitorError::InvalidResponsePayload(_bytes, e) => TmdbError::Deserialization(e),

            // We failed to even build the request, or a client hook errored.
            ProgenitorError::InvalidRequest(msg) => TmdbError::Api {
                status: 0,
                status_code: None,
                message: format!("invalid request: {msg}"),
                retry_after: None,
            },
            ProgenitorError::Custom(msg) => TmdbError::Api {
                status: 0,
                status_code: None,
                message: format!("client hook error: {msg}"),
                retry_after: None,
            },

            // A declared error response with a typed body.
            ProgenitorError::ErrorResponse(rv) => {
                let status = rv.status().as_u16();
                let retry_after = parse_retry_after(rv.headers());
                TmdbError::Api {
                    status,
                    status_code: None,
                    message: format!("{:?}", rv.into_inner()),
                    retry_after,
                }
            }

            // An undeclared error status — the common TMDB path. Read and decode
            // the `{status_code, status_message}` body ourselves.
            ProgenitorError::UnexpectedResponse(resp) => {
                let status = resp.status().as_u16();
                let retry_after = parse_retry_after(resp.headers());
                let body = resp.text().await.unwrap_or_default();
                let (status_code, message) = decode_tmdb_error_body(&body, status);
                TmdbError::Api {
                    status,
                    status_code,
                    message,
                    retry_after,
                }
            }
        }
    }
}

/// Best-effort synchronous mapping, used where an `async` body read is not
/// possible (the `?` operator). Preserves the transport/deserialization/API
/// split and the HTTP status + `Retry-After`, but cannot read an undeclared
/// error body, so an unexpected response's message is a generic
/// `unexpected response: HTTP <status>` description (a declared error response
/// uses its typed body, as `TmdbError::from_progenitor` does).
///
/// Progenitor's own `Display` is deliberately not used: for unexpected
/// responses it prints the `reqwest::Response` debug form, which includes the
/// request URL (and therefore a v3 `api_key` query parameter).
impl<E: std::fmt::Debug + Send + Sync + 'static> From<ProgenitorError<E>> for TmdbError {
    fn from(err: ProgenitorError<E>) -> Self {
        match err {
            ProgenitorError::CommunicationError(e)
            | ProgenitorError::ResponseBodyError(e)
            | ProgenitorError::InvalidUpgrade(e) => TmdbError::from(e),
            ProgenitorError::InvalidResponsePayload(_, e) => TmdbError::Deserialization(e),
            ProgenitorError::InvalidRequest(msg) => TmdbError::Api {
                status: 0,
                status_code: None,
                message: format!("invalid request: {msg}"),
                retry_after: None,
            },
            ProgenitorError::Custom(msg) => TmdbError::Api {
                status: 0,
                status_code: None,
                message: format!("client hook error: {msg}"),
                retry_after: None,
            },
            ProgenitorError::ErrorResponse(rv) => {
                let status = rv.status().as_u16();
                let retry_after = parse_retry_after(rv.headers());
                TmdbError::Api {
                    status,
                    status_code: None,
                    message: format!("{:?}", rv.into_inner()),
                    retry_after,
                }
            }
            ProgenitorError::UnexpectedResponse(resp) => {
                let status = resp.status().as_u16();
                let retry_after = parse_retry_after(resp.headers());
                TmdbError::Api {
                    status,
                    status_code: None,
                    message: format!("unexpected response: HTTP {status}"),
                    retry_after,
                }
            }
        }
    }
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
