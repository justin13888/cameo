//! The transport-failure payload of [`TmdbError::Http`](super::TmdbError::Http).

use std::fmt;

use super::redact_api_key;

/// A transport failure: no usable HTTP response was received.
///
/// Carried by [`TmdbError::Http`](super::TmdbError::Http). It classifies the
/// failure the way a retry layer needs ([`is_connect`](Self::is_connect),
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
pub(super) enum TransportKind {
    /// Connection-level: DNS, refused or reset connection, TLS handshake.
    Connect,
    /// A connect or total-request timeout elapsed.
    Timeout,
    /// Anything else: malformed HTTP, redirect exhaustion, a body cut short,
    /// a connection closed before any response.
    Other,
}

impl TransportError {
    pub(super) fn new(kind: TransportKind, message: &str) -> Self {
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
