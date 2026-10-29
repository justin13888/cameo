use std::time::Duration;

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
/// use cameo::core::error::ProviderError;
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
#[derive(Debug, thiserror::Error)]
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

#[cfg(feature = "tmdb")]
impl From<crate::providers::tmdb::TmdbError> for ProviderError {
    fn from(err: crate::providers::tmdb::TmdbError) -> Self {
        use crate::providers::tmdb::TmdbError;
        match err {
            TmdbError::Http(e) => ProviderError::Transport(e.to_string()),
            TmdbError::Deserialization(e) => ProviderError::Deserialization(e.to_string()),
            TmdbError::InvalidConfig(s) => ProviderError::InvalidInput(s),
            TmdbError::Api {
                status,
                message,
                retry_after,
                ..
            } => match status {
                401 | 403 => ProviderError::Auth(message),
                404 => ProviderError::NotFound,
                429 => ProviderError::RateLimited { retry_after },
                _ => ProviderError::Api { status, message },
            },
        }
    }
}

#[cfg(feature = "anilist")]
impl From<crate::providers::anilist::AniListError> for ProviderError {
    fn from(err: crate::providers::anilist::AniListError) -> Self {
        use crate::providers::anilist::AniListError;
        match err {
            AniListError::Http(e) => ProviderError::Transport(e.to_string()),
            AniListError::Deserialization(e) => ProviderError::Deserialization(e.to_string()),
            AniListError::InvalidConfig(s) => ProviderError::InvalidInput(s),
            AniListError::NotFound => ProviderError::NotFound,
            AniListError::NoData => ProviderError::Other("no data returned".to_string()),
            AniListError::Status {
                status,
                retry_after,
                message,
            } => match status {
                401 | 403 => ProviderError::Auth(message),
                404 => ProviderError::NotFound,
                429 => ProviderError::RateLimited { retry_after },
                _ => ProviderError::Api { status, message },
            },
            AniListError::GraphQL {
                ref errors,
                retry_after,
            } => {
                if errors.iter().any(|e| e.status == Some(429)) {
                    ProviderError::RateLimited { retry_after }
                } else if errors.iter().any(|e| e.status == Some(404)) {
                    ProviderError::NotFound
                } else if let Some(status) = most_severe_gql_status(errors) {
                    let message = errors
                        .iter()
                        .map(|e| e.message.as_str())
                        .collect::<Vec<_>>()
                        .join("; ");
                    ProviderError::Api { status, message }
                } else {
                    ProviderError::Other(err.to_string())
                }
            }
        }
    }
}

/// The most severe HTTP-error status among AniList GraphQL errors: the
/// highest `4xx`/`5xx` value (so `5xx` outranks `4xx`). Statuses outside that
/// range (e.g. the `200` AniList attaches to non-fatal errors) are ignored.
#[cfg(feature = "anilist")]
fn most_severe_gql_status(
    errors: &[crate::providers::anilist::error::AniListGqlError],
) -> Option<u16> {
    errors
        .iter()
        .filter_map(|e| e.status)
        .filter_map(|s| u16::try_from(s).ok())
        .filter(|s| (400..=599).contains(s))
        .max()
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

    #[cfg(feature = "tmdb")]
    #[test]
    fn maps_tmdb_error() {
        use crate::providers::tmdb::TmdbError;
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
    }

    #[cfg(feature = "anilist")]
    #[test]
    fn maps_anilist_error() {
        use crate::providers::anilist::AniListError;
        assert!(matches!(
            ProviderError::from(AniListError::NotFound),
            ProviderError::NotFound
        ));
        assert!(matches!(
            ProviderError::from(AniListError::Status {
                status: 429,
                retry_after: Some(Duration::from_secs(5)),
                message: "x".into(),
            }),
            ProviderError::RateLimited {
                retry_after: Some(_)
            }
        ));
        assert!(matches!(
            ProviderError::from(AniListError::InvalidConfig("x".into())),
            ProviderError::InvalidInput(_)
        ));
    }

    #[cfg(feature = "anilist")]
    fn gql(errors: serde_json::Value) -> crate::providers::anilist::AniListError {
        crate::providers::anilist::AniListError::GraphQL {
            errors: serde_json::from_value(errors).expect("valid GraphQL error fixture"),
            retry_after: None,
        }
    }

    #[cfg(feature = "anilist")]
    #[test]
    fn maps_anilist_graphql_5xx_to_api() {
        let err = ProviderError::from(gql(serde_json::json!([
            { "message": "Bad input", "status": 400 },
            { "message": "Internal", "status": 500 }
        ])));
        match err {
            ProviderError::Api { status, message } => {
                assert_eq!(status, 500, "5xx outranks 4xx");
                assert_eq!(message, "Bad input; Internal");
            }
            other => panic!("expected Api, got {other:?}"),
        }
    }

    #[cfg(feature = "anilist")]
    #[test]
    fn maps_anilist_graphql_special_statuses() {
        assert!(matches!(
            ProviderError::from(gql(serde_json::json!([
                { "message": "Internal", "status": 500 },
                { "message": "Too Many Requests.", "status": 429 }
            ]))),
            ProviderError::RateLimited { .. }
        ));
        assert!(matches!(
            ProviderError::from(gql(serde_json::json!([
                { "message": "Not Found.", "status": 404 }
            ]))),
            ProviderError::NotFound
        ));
        // No HTTP-error status at all stays `Other`.
        assert!(matches!(
            ProviderError::from(gql(serde_json::json!([
                { "message": "no status" },
                { "message": "non-fatal", "status": 200 }
            ]))),
            ProviderError::Other(_)
        ));
    }
}
