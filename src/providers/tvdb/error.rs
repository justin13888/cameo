use std::time::Duration;

use crate::{
    core::error::ProviderError,
    providers::{http::is_retryable_status, retry::Retryable},
};

/// Error type for the TheTVDB provider.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum TvdbError {
    /// HTTP transport error (connection, DNS, TLS) — no response was received.
    ///
    /// The request URL is stripped with [`reqwest::Error::without_url`], so a
    /// search query never travels into logs through an error.
    #[error("HTTP transport error: {0}")]
    Http(#[source] reqwest::Error),

    /// TheTVDB answered with an error HTTP status.
    ///
    /// A `401` here means the bearer token was refused even after one fresh
    /// login, or that `POST /login` itself rejected the API key or PIN.
    #[error("TheTVDB HTTP error {status}: {message}")]
    Status {
        /// HTTP response status code.
        status: u16,
        /// The `Retry-After` delay, if the response supplied one.
        retry_after: Option<Duration>,
        /// TheTVDB's `message`, or a truncated body when it sent none.
        message: String,
    },

    /// A response body could not be deserialized.
    #[error("deserialization error: {0}")]
    Deserialization(#[from] serde_json::Error),

    /// A successful response carried no `data`.
    #[error("no data returned from TheTVDB")]
    NoData,

    /// The requested record does not exist (HTTP `404`, or an episode or
    /// season the series does not have).
    #[error("not found")]
    NotFound,

    /// Invalid configuration (rejected at construction time).
    #[error("invalid configuration: {0}")]
    InvalidConfig(String),
}

/// Wrap a transport error, stripping its request URL.
impl From<reqwest::Error> for TvdbError {
    fn from(err: reqwest::Error) -> Self {
        TvdbError::Http(err.without_url())
    }
}

impl Retryable for TvdbError {
    /// A 429/5xx status, or a connect/timeout transport error.
    fn is_retryable(&self) -> bool {
        match self {
            TvdbError::Status { status, .. } => is_retryable_status(*status),
            TvdbError::Http(e) => e.is_timeout() || e.is_connect(),
            _ => false,
        }
    }

    fn retry_after(&self) -> Option<Duration> {
        match self {
            TvdbError::Status { retry_after, .. } => *retry_after,
            _ => None,
        }
    }
}

impl From<TvdbError> for ProviderError {
    fn from(err: TvdbError) -> Self {
        match err {
            TvdbError::Http(e) => ProviderError::Transport(e.without_url().to_string()),
            TvdbError::Status {
                status,
                retry_after,
                message,
            } => ProviderError::from_http_status(status, message, retry_after),
            TvdbError::Deserialization(e) => ProviderError::Deserialization(e.to_string()),
            TvdbError::NoData => ProviderError::Other("no data returned".to_string()),
            TvdbError::NotFound => ProviderError::NotFound,
            TvdbError::InvalidConfig(s) => ProviderError::InvalidInput(s),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::http::test_support::{json_error, reqwest_builder_error};

    fn status(code: u16) -> TvdbError {
        TvdbError::Status {
            status: code,
            retry_after: Some(Duration::from_secs(3)),
            message: "msg".into(),
        }
    }

    #[test]
    fn maps_every_tvdb_error_branch() {
        for code in [401, 403] {
            assert_eq!(
                ProviderError::from(status(code)),
                ProviderError::Auth("msg".into())
            );
        }
        assert_eq!(ProviderError::from(status(404)), ProviderError::NotFound);
        assert_eq!(
            ProviderError::from(status(429)),
            ProviderError::RateLimited {
                retry_after: Some(Duration::from_secs(3))
            }
        );
        assert_eq!(
            ProviderError::from(status(500)),
            ProviderError::Api {
                status: 500,
                message: "msg".into()
            }
        );

        let http = reqwest_builder_error();
        let http_text = http.to_string();
        assert_eq!(
            ProviderError::from(TvdbError::Http(http)),
            ProviderError::Transport(http_text)
        );
        let json = json_error();
        let json_text = json.to_string();
        assert_eq!(
            ProviderError::from(TvdbError::Deserialization(json)),
            ProviderError::Deserialization(json_text)
        );
        assert_eq!(
            ProviderError::from(TvdbError::NoData),
            ProviderError::Other("no data returned".into())
        );
        assert_eq!(
            ProviderError::from(TvdbError::NotFound),
            ProviderError::NotFound
        );
        assert_eq!(
            ProviderError::from(TvdbError::InvalidConfig("x".into())),
            ProviderError::InvalidInput("x".into())
        );
    }

    #[test]
    fn retries_only_transient_statuses() {
        for code in [429, 500, 503] {
            assert!(status(code).is_retryable(), "{code}");
        }
        for code in [400, 401, 404] {
            assert!(!status(code).is_retryable(), "{code}");
        }
        assert!(!TvdbError::NotFound.is_retryable());
        assert_eq!(status(429).retry_after(), Some(Duration::from_secs(3)));
        assert_eq!(TvdbError::NoData.retry_after(), None);
    }
}
