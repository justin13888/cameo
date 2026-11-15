//! The [`TmdbClient`] itself, with its endpoints split by family into the
//! submodules below (mirroring `unified::facade`).

mod details;
mod discovery;
mod external_ids;
mod recommendations;
mod search;
mod seasons;
mod watch_providers;

use super::{
    config::{TmdbAuth, TmdbConfig},
    convert::MAX_PAGE,
    error::{RequestErrorKind, TmdbError},
};
use crate::{
    generated::tmdb::{self, Credential, SecretString},
    providers::{rate_limit::RateLimiter, retry},
    unified::conversions::native_id,
};

/// The security scheme a v4 bearer token is registered under
/// (`scripts/tmdb-spec-patch.jq` declares it).
const V4_BEARER_SCHEME: &str = "v4_bearer";

/// The security scheme a v3 API key is registered under
/// (`scripts/tmdb-spec-patch.jq` declares it).
const V3_API_KEY_SCHEME: &str = "v3_api_key";

/// High-level TMDB API client wrapping the generated (spargen) client.
///
/// Adds bearer token authentication, rate limiting, and ergonomic pagination.
///
/// # Ids
///
/// Id parameters are `u64`, the type TMDB ids carry in every result (e.g.
/// [`CastMember::id`](super::models::CastMember::id)). TMDB's wire format holds
/// ids, season numbers, and episode numbers as `i32`, so any of them above
/// `i32::MAX` fails with [`TmdbError::Request`]
/// ([`RequestErrorKind::InvalidRequest`]) before anything is sent, rather than
/// being wrapped or clamped onto a different record.
///
/// # Cloning
///
/// `TmdbClient` is cheaply cloneable — the underlying HTTP client,
/// configuration, and rate limiter are all reference-counted so clones share
/// the same connection pool and the same rate-limit budget.
///
/// # Debug output
///
/// The `Debug` output never contains the API credential: it shows the
/// (redacted) [`TmdbConfig`] and the rate limiter, and omits the underlying
/// generated client (whose own `Debug` redacts the registered credential).
#[derive(Clone)]
pub struct TmdbClient {
    inner: tmdb::Client,
    config: TmdbConfig,
    rate_limiter: RateLimiter,
}

impl std::fmt::Debug for TmdbClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TmdbClient")
            .field("config", &self.config)
            .field("rate_limiter", &self.rate_limiter)
            .finish_non_exhaustive()
    }
}

impl TmdbClient {
    /// Create a new TMDB client from the given configuration.
    pub fn new(config: TmdbConfig) -> Result<Self, TmdbError> {
        config.validate().map_err(TmdbError::InvalidConfig)?;

        if let TmdbAuth::V4Bearer(token) = &config.auth {
            // Reject a token that cannot travel in a header up front, rather
            // than failing every request.
            reqwest::header::HeaderValue::from_str(&format!("Bearer {token}"))
                .map_err(|e| TmdbError::InvalidConfig(format!("invalid API token: {e}")))?;
        }

        let http_client = crate::providers::http::build_client(
            config.connect_timeout,
            config.request_timeout,
            reqwest::header::HeaderMap::new(),
        )
        .map_err(TmdbError::from_reqwest)?;

        let base_url = config.base_url.as_str();
        let inner = tmdb::Client::with_client(http_client, base_url)
            .map_err(|e| TmdbError::InvalidConfig(format!("invalid base URL: {e}")))?;
        // The spec patch declares both credentials as security schemes: a v4
        // token travels as `Authorization: Bearer`, a v3 key as the `api_key`
        // query parameter. The generated client redacts either in `Debug`.
        let inner = match &config.auth {
            TmdbAuth::V4Bearer(token) => inner.with_credential(
                V4_BEARER_SCHEME,
                Credential::Bearer(SecretString::from(token.clone())),
            ),
            TmdbAuth::V3ApiKey(key) => inner.with_credential(
                V3_API_KEY_SCHEME,
                Credential::ApiKey(SecretString::from(key.clone())),
            ),
        };
        let rate_limiter = RateLimiter::new(config.rate_limit);

        Ok(Self {
            inner,
            config,
            rate_limiter,
        })
    }

    /// Returns a reference to the client configuration.
    pub fn config(&self) -> &TmdbConfig {
        &self.config
    }

    // ── Helper accessors for default config values ──

    fn language(&self) -> Option<String> {
        self.config.language.clone()
    }

    fn region(&self) -> Option<String> {
        self.config.region.clone()
    }

    fn include_adult(&self) -> Option<bool> {
        self.config.include_adult
    }

    /// Clamp a page to TMDB's supported `1..=MAX_PAGE` range and convert to
    /// the generated client's `i32` without a wrapping cast.
    fn tmdb_page(page: Option<u32>) -> Option<i32> {
        page.map(|p| p.clamp(1, MAX_PAGE) as i32)
    }

    /// Execute a generated-client call under the rate limiter, unwrap the
    /// response body, and map any failure into a structured [`TmdbError`]
    /// (decoding TMDB's `{status_code, status_message}` body and `Retry-After`
    /// header via [`TmdbError::from_generated`]).
    ///
    /// `make_call` is a closure returning a fresh request future each time it is
    /// invoked, so the retry loop can re-issue it. This is the single seam
    /// through which every endpoint runs — rate limiting and retries are
    /// applied here rather than at each call site. Retries re-acquire the rate
    /// limiter, so they are paced like any other request.
    async fn execute<T, E, F, Fut>(&self, make_call: F) -> Result<T, TmdbError>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<tmdb::ResponseValue<T>, tmdb::Error<E>>>,
        E: std::fmt::Debug,
    {
        retry::run(self.config.retry, &self.rate_limiter, "tmdb", || async {
            make_call()
                .await
                .map(tmdb::ResponseValue::into_inner)
                .map_err(TmdbError::from_generated)
        })
        .await
    }
}

/// The id a details response is converted under: the id TMDB reported when it
/// is valid, otherwise (missing, zero, or negative) the requested id.
fn details_id(reported: &mut Option<i64>, requested: i32) {
    if reported.and_then(native_id).is_none() {
        *reported = Some(i64::from(requested));
    }
}

/// Convert a caller's id or number to the `i32` TMDB's wire format carries,
/// without wrapping or clamping.
///
/// `what` names the parameter in the error message (e.g. `"id"`).
///
/// # Errors
///
/// [`TmdbError::Request`] ([`RequestErrorKind::InvalidRequest`]) when `value`
/// is above `i32::MAX`: TMDB cannot represent it, so nothing is sent.
pub(crate) fn wire_int(value: u64, what: &str) -> Result<i32, TmdbError> {
    i32::try_from(value).map_err(|_| TmdbError::Request {
        kind: RequestErrorKind::InvalidRequest,
        message: format!("TMDB {what} must be at most {}, got {value}", i32::MAX),
    })
}

/// A requested TMDB id as the `i32` the generated client sends.
///
/// # Errors
///
/// [`TmdbError::Request`] ([`RequestErrorKind::InvalidRequest`]) when `id` is
/// zero (no TMDB record has such an id) or above `i32::MAX` (TMDB cannot
/// represent it), so nothing is sent.
fn requested_id(id: u64) -> Result<i32, TmdbError> {
    if id == 0 {
        return Err(TmdbError::Request {
            kind: RequestErrorKind::InvalidRequest,
            message: "TMDB ids are positive integers, got 0".to_owned(),
        });
    }
    wire_int(id, "id")
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECRET: &str = "SECRETKEY123";

    /// The generated client's `Debug` must not expose the credential for
    /// either auth scheme, while still showing which scheme is registered.
    #[test]
    fn generated_client_debug_redacts_credentials() {
        for (config, scheme) in [
            (TmdbConfig::new(SECRET), V4_BEARER_SCHEME),
            (TmdbConfig::new_v3(SECRET), V3_API_KEY_SCHEME),
        ] {
            let client = TmdbClient::new(config).expect("valid config");
            for debug in [
                format!("{:?}", client.inner),
                format!("{:#?}", client.inner),
            ] {
                assert!(!debug.contains(SECRET), "credential leaked: {debug}");
                assert!(debug.contains(scheme), "{debug}");
            }
        }
    }

    /// The `_url` accessors never carry the v3 key: it is attached only when a
    /// request is sent.
    #[test]
    fn v3_key_is_not_in_url_accessor() {
        let client = TmdbClient::new(TmdbConfig::new_v3(SECRET)).expect("valid config");
        let url = client
            .inner
            .movie_details_url(550, None)
            .expect("valid url");
        assert_eq!(url.path(), "/3/movie/550");
        assert!(!url.as_str().contains(SECRET), "{url}");
    }

    /// An invalid id in a details response gives way to the requested id; a
    /// valid one is kept.
    #[test]
    fn details_id_replaces_only_invalid_ids() {
        for reported in [None, Some(0), Some(-5)] {
            let mut id = reported;
            details_id(&mut id, 550);
            assert_eq!(id, Some(550), "reported {reported:?}");
        }
        let mut id = Some(603);
        details_id(&mut id, 550);
        assert_eq!(id, Some(603));
    }

    /// A value TMDB can carry converts unchanged; one above `i32::MAX` is
    /// rejected rather than wrapped or clamped.
    #[test]
    fn wire_int_rejects_values_above_i32_max() {
        let max = u64::try_from(i32::MAX).expect("positive");
        assert_eq!(wire_int(0, "id").expect("in range"), 0);
        assert_eq!(wire_int(max, "id").expect("in range"), i32::MAX);
        for value in [max + 1, u64::MAX] {
            let err = wire_int(value, "season number").expect_err("out of range");
            assert!(
                matches!(
                    &err,
                    TmdbError::Request {
                        kind: RequestErrorKind::InvalidRequest,
                        message,
                    } if message.contains("season number") && message.contains(&value.to_string())
                ),
                "got {err:?}"
            );
        }
    }

    /// A zero or out-of-range requested id is rejected before anything is
    /// sent.
    #[test]
    fn requested_id_rejects_zero_and_out_of_range_ids() {
        assert_eq!(requested_id(550).expect("valid id"), 550);
        let max = u64::try_from(i32::MAX).expect("positive");
        assert_eq!(requested_id(max).expect("valid id"), i32::MAX);
        for id in [0, max + 1, u64::MAX] {
            let err = requested_id(id).expect_err("invalid id");
            assert!(
                matches!(
                    err,
                    TmdbError::Request {
                        kind: RequestErrorKind::InvalidRequest,
                        ..
                    }
                ),
                "got {err:?}"
            );
        }
    }
}
