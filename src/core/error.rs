use std::{fmt, sync::Arc, time::Duration};

/// The unified error type returned by provider trait implementations.
///
/// Per ADR 0001, provider traits return this single concrete type (rather than a
/// per-provider associated error) so they are `dyn`-compatible. Each provider
/// maps its native error (`TmdbError`, `AniListError`) into `ProviderError` via
/// `From`.
///
/// # Matching on variants
///
/// ```
/// use cameo::ProviderError;
///
/// fn handle(err: ProviderError) {
///     match err {
///         ProviderError::NotFound => eprintln!("not found"),
///         ProviderError::RateLimited { retry_after } => {
///             eprintln!("rate limited; retry after {retry_after:?}");
///         }
///         ProviderError::Api { status, message } => eprintln!("HTTP {status}: {message}"),
///         other => eprintln!("{other}"),
///     }
/// }
/// ```
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ProviderError {
    /// The requested resource was not found.
    #[error("resource not found")]
    NotFound,

    /// The provider does not support this operation.
    #[error("operation not supported by this provider")]
    Unsupported,

    /// The provider rate-limited the request.
    #[error("rate limited{}", .retry_after.map(|d| format!(" (retry after {}s)", d.as_secs())).unwrap_or_default())]
    RateLimited {
        /// The server-requested wait before retrying, if provided.
        retry_after: Option<Duration>,
    },

    /// Authentication failed (bad or missing credentials).
    #[error("authentication failed: {0}")]
    Auth(String),

    /// The provider returned an error status.
    #[error("API error (HTTP {status}): {message}")]
    Api {
        /// HTTP status code.
        status: u16,
        /// Human-readable message.
        message: String,
    },

    /// A transport-level failure (connection, DNS, TLS, timeout).
    #[error("transport error: {0}")]
    Transport(String),

    /// A response body could not be deserialized.
    #[error("deserialization error: {0}")]
    Deserialization(String),

    /// The caller supplied invalid input or configuration.
    #[error("invalid input: {0}")]
    InvalidInput(String),

    /// Any other provider error.
    #[error("provider error: {0}")]
    Other(String),
}

impl ProviderError {
    /// Classify an HTTP error `status`: `401`/`403` is [`Auth`](Self::Auth)
    /// carrying `message`, `404` is [`NotFound`](Self::NotFound), `429` is
    /// [`RateLimited`](Self::RateLimited) carrying `retry_after`, and any
    /// other status is [`Api`](Self::Api).
    pub(crate) fn from_http_status(
        status: u16,
        message: String,
        retry_after: Option<Duration>,
    ) -> Self {
        match status {
            401 | 403 => ProviderError::Auth(message),
            404 => ProviderError::NotFound,
            429 => ProviderError::RateLimited { retry_after },
            _ => ProviderError::Api { status, message },
        }
    }
}

/// The shared, type-erased source of a [`TransportError`] or [`DecodeError`].
type Source = Arc<dyn std::error::Error + Send + Sync + 'static>;

/// A transport failure: no usable HTTP response was received.
///
/// Carried by `TmdbError::Http`, `AniListError::Http` and `TvdbError::Http`. It classifies the
/// failure the way a retry layer needs ([`is_connect`](Self::is_connect),
/// [`is_timeout`](Self::is_timeout)) and describes it in its `Display` output.
///
/// The HTTP client's own error type is not part of cameo's public API, so a
/// major upgrade of that client is not a cameo breaking change. Where the
/// original error can be kept without its request URL, it is the
/// [`source`](std::error::Error::source) (downcast it to inspect it); a TMDB
/// failure raised inside the generated client has no source, because that
/// original cannot be separated from a URL that carries a v3 `api_key`.
/// Neither the description nor the source ever holds the request URL's
/// `api_key` query value.
#[derive(Debug, Clone)]
pub struct TransportError {
    kind: TransportKind,
    message: String,
    source: Option<Source>,
}

/// The class of a [`TransportError`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    not(any(feature = "tmdb", feature = "anilist", feature = "tvdb")),
    allow(dead_code)
)]
pub(crate) enum TransportKind {
    /// Connection-level: DNS, refused or reset connection, TLS handshake.
    Connect,
    /// A connect or total-request timeout elapsed.
    Timeout,
    /// Anything else: malformed HTTP, redirect exhaustion, a body cut short,
    /// a connection closed before any response.
    Other,
}

impl TransportError {
    /// A transport error of `kind` described by `message`, with no source.
    ///
    /// The caller redacts `message`: it is printed as given.
    #[cfg_attr(not(feature = "tmdb"), allow(dead_code))]
    pub(crate) fn new(kind: TransportKind, message: String) -> Self {
        Self {
            kind,
            message,
            source: None,
        }
    }

    /// A transport error wrapping `err`, classified from it and described by
    /// `describe`. The request URL is stripped from `err` before `describe`
    /// sees it and before it is kept as the [`source`](std::error::Error::source).
    #[cfg_attr(
        not(any(feature = "tmdb", feature = "anilist", feature = "tvdb")),
        allow(dead_code)
    )]
    pub(crate) fn from_reqwest(
        err: reqwest::Error,
        describe: impl FnOnce(&reqwest::Error) -> String,
    ) -> Self {
        let kind = if err.is_timeout() {
            TransportKind::Timeout
        } else if err.is_connect() {
            TransportKind::Connect
        } else {
            TransportKind::Other
        };
        let err = err.without_url();
        Self {
            kind,
            message: describe(&err),
            source: Some(Arc::new(err)),
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

impl std::error::Error for TransportError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source
            .as_deref()
            .map(|e| e as &(dyn std::error::Error + 'static))
    }
}

/// A body or stored value that could not be decoded from (or encoded to) its
/// JSON form.
///
/// Carried by `TmdbError::Deserialization`, `AniListError::Deserialization`,
/// `TvdbError::Deserialization` and `CacheError::Serialization`. The JSON library's own error type is not
/// part of cameo's public API, so a major upgrade of that library is not a
/// cameo breaking change. Where a JSON parser error produced it, that error is
/// the [`source`](std::error::Error::source) (downcast it to inspect it); a
/// decode failure the generated TMDB client reports carries only its message.
#[derive(Debug, Clone)]
pub struct DecodeError {
    message: String,
    source: Option<Source>,
}

impl DecodeError {
    /// A decode error described by `message`, with no source.
    #[cfg_attr(not(feature = "tmdb"), allow(dead_code))]
    pub(crate) fn new(message: String) -> Self {
        Self {
            message,
            source: None,
        }
    }

    /// A decode error wrapping a JSON parser error, which becomes its
    /// [`source`](std::error::Error::source).
    #[cfg_attr(
        not(any(feature = "anilist", feature = "tvdb", feature = "cache")),
        allow(dead_code)
    )]
    pub(crate) fn json(err: serde_json::Error) -> Self {
        Self {
            message: err.to_string(),
            source: Some(Arc::new(err)),
        }
    }
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for DecodeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source
            .as_deref()
            .map(|e| e as &(dyn std::error::Error + 'static))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every public `ProviderError` variant is constructible.
    #[test]
    fn all_variants_construct() {
        let variants = [
            ProviderError::NotFound,
            ProviderError::Unsupported,
            ProviderError::RateLimited {
                retry_after: Some(Duration::from_secs(1)),
            },
            ProviderError::Auth("bad token".into()),
            ProviderError::Api {
                status: 500,
                message: "boom".into(),
            },
            ProviderError::Transport("connection reset".into()),
            ProviderError::Deserialization("bad json".into()),
            ProviderError::InvalidInput("negative id".into()),
            ProviderError::Other("misc".into()),
        ];
        // Each renders without panicking.
        for v in &variants {
            assert!(!v.to_string().is_empty());
        }
    }

    /// Every `ProviderError` variant renders its own exact message.
    #[test]
    fn every_variant_display_is_pinned() {
        let cases = [
            (ProviderError::NotFound, "resource not found"),
            (
                ProviderError::Unsupported,
                "operation not supported by this provider",
            ),
            (
                ProviderError::RateLimited { retry_after: None },
                "rate limited",
            ),
            (
                ProviderError::RateLimited {
                    retry_after: Some(Duration::from_secs(7)),
                },
                "rate limited (retry after 7s)",
            ),
            (
                ProviderError::Auth("bad token".into()),
                "authentication failed: bad token",
            ),
            (
                ProviderError::Api {
                    status: 500,
                    message: "boom".into(),
                },
                "API error (HTTP 500): boom",
            ),
            (
                ProviderError::Transport("reset".into()),
                "transport error: reset",
            ),
            (
                ProviderError::Deserialization("bad json".into()),
                "deserialization error: bad json",
            ),
            (
                ProviderError::InvalidInput("negative id".into()),
                "invalid input: negative id",
            ),
            (ProviderError::Other("misc".into()), "provider error: misc"),
        ];
        for (err, expected) in cases {
            assert_eq!(err.to_string(), expected);
        }
    }

    /// `from_http_status` classifies every status branch, carrying the
    /// message or `retry_after`.
    #[test]
    fn from_http_status_classifies_each_branch() {
        let retry = Some(Duration::from_secs(4));
        let map = |status| ProviderError::from_http_status(status, "msg".into(), retry);
        for status in [401, 403] {
            assert_eq!(map(status), ProviderError::Auth("msg".into()), "{status}");
        }
        assert_eq!(map(404), ProviderError::NotFound);
        assert_eq!(map(429), ProviderError::RateLimited { retry_after: retry });
        for status in [400, 418, 500, 503] {
            assert_eq!(
                map(status),
                ProviderError::Api {
                    status,
                    message: "msg".into()
                },
                "{status}"
            );
        }
    }

    /// A `reqwest::Error` whose request URL carries a v3 `api_key`: a send to
    /// a loopback port nothing listens on, refused without leaving the host.
    async fn reqwest_error_with_url() -> reqwest::Error {
        match reqwest::Client::new()
            .get("http://127.0.0.1:1/3/movie/1?api_key=SECRET")
            .send()
            .await
        {
            Ok(response) => panic!("expected a send error, got {response:?}"),
            Err(e) => e,
        }
    }

    /// `from_reqwest` keeps the original as `source()`, stripped of its URL,
    /// classifies it, and describes it through the caller's closure.
    #[tokio::test]
    async fn transport_error_keeps_url_free_reqwest_source() {
        let original = reqwest_error_with_url().await;
        assert!(original.url().is_some(), "fixture must carry a URL");
        let err = TransportError::from_reqwest(original, |e| format!("described: {e}"));

        assert!(err.to_string().starts_with("described: "), "{err}");
        assert!(err.is_connect() && !err.is_timeout(), "{err:?}");
        let source = std::error::Error::source(&err).expect("source");
        let inner = source
            .downcast_ref::<reqwest::Error>()
            .expect("source downcasts to reqwest::Error");
        assert!(inner.url().is_none(), "source kept its URL: {inner:?}");
        assert!(!format!("{err:?}").contains("SECRET"), "{err:?}");
        // Clone shares the source.
        assert!(std::error::Error::source(&err.clone()).is_some());
    }

    /// `TransportError::new` classifies by its kind and has no source.
    #[test]
    fn transport_error_new_has_kind_and_no_source() {
        let err = TransportError::new(TransportKind::Timeout, "slow".into());
        assert!(err.is_timeout() && !err.is_connect());
        assert_eq!(err.to_string(), "slow");
        assert!(std::error::Error::source(&err).is_none());
        let err = TransportError::new(TransportKind::Connect, "refused".into());
        assert!(err.is_connect() && !err.is_timeout());
    }

    /// `DecodeError::json` prints the parser's message and returns the parser
    /// error from `source()`; `DecodeError::new` has no source.
    #[test]
    fn decode_error_source_is_the_json_error() {
        let Err(json) = serde_json::from_str::<u32>("not json") else {
            panic!("expected a JSON error");
        };
        let text = json.to_string();
        let err = DecodeError::json(json);
        assert_eq!(err.to_string(), text);
        let source = std::error::Error::source(&err).expect("source");
        assert!(source.downcast_ref::<serde_json::Error>().is_some());
        assert!(std::error::Error::source(&err.clone()).is_some());

        let err = DecodeError::new("results[0].id".into());
        assert_eq!(err.to_string(), "results[0].id");
        assert!(std::error::Error::source(&err).is_none());
    }
}
