//! Retry policy for transient provider failures.
//!
//! Retries 429/5xx and connect/transport errors with jittered exponential
//! backoff, honoring a server-supplied `Retry-After` when present. Both
//! providers run their requests through one crate-internal loop, which
//! classifies each provider's errors through a crate-internal trait. The loop re-acquires the rate limiter
//! before every attempt, so retries cannot stampede.

use std::{fmt, future::Future, time::Duration};

use super::rate_limit::RateLimiter;

/// A provider error the retry loop can classify.
pub(crate) trait Retryable: fmt::Display {
    /// Whether this failure is transient and worth retrying.
    fn is_retryable(&self) -> bool;

    /// The server-supplied `Retry-After` delay, if any.
    fn retry_after(&self) -> Option<Duration>;
}

/// Run `attempt` under `limiter`, retrying transient failures per `policy`.
///
/// Every attempt (the first and each retry) first waits for a rate-limit
/// slot. A failure is retried while `policy` permits another retry and the
/// error [`is_retryable`](Retryable::is_retryable), after a
/// [`backoff`](RetryPolicy::backoff) that honors its
/// [`retry_after`](Retryable::retry_after). Each retry logs
/// `"{label}: retrying transient error"` at debug level. The last error is
/// returned unchanged.
pub(crate) async fn run<T, E, F, Fut>(
    policy: RetryPolicy,
    limiter: &RateLimiter,
    label: &str,
    mut attempt: F,
) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, E>>,
    E: Retryable,
{
    let mut retry_index = 0u32;
    loop {
        limiter.acquire().await;
        match attempt().await {
            Ok(value) => return Ok(value),
            Err(err) => {
                if policy.should_retry(retry_index) && err.is_retryable() {
                    let delay = policy.backoff(retry_index, err.retry_after());
                    tracing::debug!(
                        retry = retry_index,
                        ?delay,
                        error = %err,
                        "{label}: retrying transient error"
                    );
                    tokio::time::sleep(delay).await;
                    retry_index += 1;
                    continue;
                }
                return Err(err);
            }
        }
    }
}

/// A hard ceiling on how long we will wait for a single `Retry-After`, so a
/// pathological value cannot hang a request indefinitely.
const RETRY_AFTER_CAP: Duration = Duration::from_secs(300);

/// Configuration for retrying transient failures.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryPolicy {
    /// Maximum number of **retries** after the initial attempt (0 disables retrying).
    pub max_retries: u32,
    /// Base backoff delay for the first retry.
    pub base_delay: Duration,
    /// Upper bound on the exponential backoff (does not cap `Retry-After`).
    pub max_delay: Duration,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_delay: Duration::from_millis(500),
            max_delay: Duration::from_secs(30),
        }
    }
}

impl RetryPolicy {
    /// A policy that never retries.
    pub const fn disabled() -> Self {
        Self {
            max_retries: 0,
            base_delay: Duration::ZERO,
            max_delay: Duration::ZERO,
        }
    }

    /// A custom policy.
    pub const fn new(max_retries: u32, base_delay: Duration, max_delay: Duration) -> Self {
        Self {
            max_retries,
            base_delay,
            max_delay,
        }
    }

    /// Whether a retry numbered `retry_index` (0-based) is still permitted.
    pub(crate) fn should_retry(&self, retry_index: u32) -> bool {
        retry_index < self.max_retries
    }

    /// The delay before the retry numbered `retry_index` (0-based).
    ///
    /// A server-supplied `retry_after` is authoritative and honored directly
    /// (plus a little jitter, capped at [`RETRY_AFTER_CAP`]); otherwise the
    /// delay is exponential (`base_delay * 2^retry_index`, capped at
    /// `max_delay`) with equal jitter so concurrent callers desynchronize.
    pub(crate) fn backoff(&self, retry_index: u32, retry_after: Option<Duration>) -> Duration {
        if let Some(ra) = retry_after {
            let jitter = Duration::from_millis((jitter_fraction() * 250.0) as u64);
            return ra.saturating_add(jitter).min(RETRY_AFTER_CAP);
        }
        let factor = 2u32.saturating_pow(retry_index);
        let capped = self.base_delay.saturating_mul(factor).min(self.max_delay);
        // Equal jitter: half of the delay is fixed, half is random.
        let half = capped / 2;
        half + half.mul_f64(jitter_fraction())
    }
}

/// A pseudo-random fraction in `[0, 1)` for backoff jitter, without pulling in a
/// random-number dependency. `RandomState` is seeded per construction, so
/// successive calls differ.
fn jitter_fraction() -> f64 {
    use std::{
        collections::hash_map::RandomState,
        hash::{BuildHasher, Hasher},
    };
    let mut hasher = RandomState::new().build_hasher();
    hasher.write_u64(0x9E37_79B9_7F4A_7C15);
    // Take the top 53 bits for a uniform double in [0, 1).
    ((hasher.finish() >> 11) as f64) / ((1u64 << 53) as f64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retry_after_is_honored() {
        let policy = RetryPolicy::default();
        let delay = policy.backoff(0, Some(Duration::from_secs(7)));
        // At least the server-requested wait, plus a small jitter margin.
        assert!(delay >= Duration::from_secs(7), "got {delay:?}");
        assert!(delay <= Duration::from_secs(8), "got {delay:?}");
    }

    #[test]
    fn retry_after_is_capped() {
        let policy = RetryPolicy::default();
        let delay = policy.backoff(0, Some(Duration::from_secs(10_000)));
        assert_eq!(delay, RETRY_AFTER_CAP);
    }

    #[test]
    fn exponential_backoff_grows_and_caps() {
        let policy = RetryPolicy::new(10, Duration::from_millis(100), Duration::from_millis(800));
        // Equal jitter keeps each delay within [capped/2, capped].
        for (idx, capped_ms) in [(0u32, 100u64), (1, 200), (2, 400), (3, 800), (4, 800)] {
            let d = policy.backoff(idx, None);
            assert!(
                d >= Duration::from_millis(capped_ms / 2) && d <= Duration::from_millis(capped_ms),
                "retry {idx}: {d:?} outside [{}/2, {capped_ms}]ms",
                capped_ms
            );
        }
    }

    proptest::proptest! {
        /// Without `Retry-After`, every delay lies in `[capped / 2, capped]`
        /// where `capped = min(base * 2^retry, max_delay)`.
        #[test]
        fn exponential_backoff_stays_within_equal_jitter_bounds(
            base_ms in 0u64..10_000,
            max_ms in 0u64..120_000,
            retry_index in 0u32..64,
        ) {
            let base = Duration::from_millis(base_ms);
            let max = Duration::from_millis(max_ms);
            let policy = RetryPolicy::new(u32::MAX, base, max);
            let capped = base.saturating_mul(2u32.saturating_pow(retry_index)).min(max);
            let delay = policy.backoff(retry_index, None);
            proptest::prop_assert!(delay >= capped / 2, "{delay:?} < {capped:?} / 2");
            proptest::prop_assert!(delay <= capped, "{delay:?} > {capped:?}");
        }

        /// A `Retry-After` is honoured with under 250ms of jitter, and never
        /// exceeds the 300s cap.
        #[test]
        fn retry_after_stays_within_jitter_and_cap(
            retry_after_ms in 0u64..1_000_000,
            retry_index in 0u32..64,
        ) {
            let retry_after = Duration::from_millis(retry_after_ms);
            let delay = RetryPolicy::default().backoff(retry_index, Some(retry_after));
            let floor = retry_after.min(RETRY_AFTER_CAP);
            let ceiling = (retry_after + Duration::from_millis(250)).min(RETRY_AFTER_CAP);
            proptest::prop_assert!(delay >= floor, "{delay:?} < {floor:?}");
            proptest::prop_assert!(delay <= ceiling, "{delay:?} > {ceiling:?}");
        }
    }

    /// A test error: retryable or not, with an optional `Retry-After`.
    #[derive(Debug, PartialEq, Eq)]
    struct TestError {
        retryable: bool,
        retry_after: Option<Duration>,
    }

    impl fmt::Display for TestError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("test error")
        }
    }

    impl Retryable for TestError {
        fn is_retryable(&self) -> bool {
            self.retryable
        }
        fn retry_after(&self) -> Option<Duration> {
            self.retry_after
        }
    }

    fn unlimited() -> RateLimiter {
        RateLimiter::new(crate::providers::rate_limit::RateLimit::Unlimited)
    }

    /// Runs `run` with an attempt that fails with `err()` on its first
    /// `failures` calls, then succeeds; returns the outcome and the call count.
    async fn run_failing(
        policy: RetryPolicy,
        failures: u32,
        err: fn() -> TestError,
    ) -> (Result<u32, TestError>, u32) {
        let calls = std::cell::Cell::new(0u32);
        let outcome = run(policy, &unlimited(), "test", || {
            let n = calls.get() + 1;
            calls.set(n);
            async move { if n <= failures { Err(err()) } else { Ok(n) } }
        })
        .await;
        (outcome, calls.get())
    }

    fn transient() -> TestError {
        TestError {
            retryable: true,
            retry_after: None,
        }
    }

    #[tokio::test(start_paused = true)]
    async fn run_retries_transient_errors_until_success() {
        let (outcome, calls) = run_failing(RetryPolicy::default(), 2, transient).await;
        assert_eq!(outcome, Ok(3));
        assert_eq!(calls, 3);
    }

    #[tokio::test(start_paused = true)]
    async fn run_stops_after_max_retries_with_last_error() {
        let policy = RetryPolicy::new(2, Duration::from_millis(10), Duration::from_millis(10));
        let (outcome, calls) = run_failing(policy, u32::MAX, transient).await;
        assert_eq!(outcome, Err(transient()));
        assert_eq!(calls, 3, "initial attempt plus two retries");
    }

    #[tokio::test(start_paused = true)]
    async fn run_does_not_retry_non_retryable_errors() {
        let fatal = || TestError {
            retryable: false,
            retry_after: None,
        };
        let (outcome, calls) = run_failing(RetryPolicy::default(), u32::MAX, fatal).await;
        assert_eq!(outcome, Err(fatal()));
        assert_eq!(calls, 1);
    }

    #[tokio::test(start_paused = true)]
    async fn run_waits_for_retry_after_before_retrying() {
        let limited = || TestError {
            retryable: true,
            retry_after: Some(Duration::from_secs(5)),
        };
        let start = tokio::time::Instant::now();
        let (outcome, calls) = run_failing(RetryPolicy::default(), 1, limited).await;
        assert_eq!(outcome, Ok(2));
        assert_eq!(calls, 2);
        assert!(start.elapsed() >= Duration::from_secs(5));
    }

    #[test]
    fn disabled_policy_never_retries() {
        let policy = RetryPolicy::disabled();
        assert!(!policy.should_retry(0));
    }

    #[test]
    fn should_retry_respects_max() {
        let policy = RetryPolicy::new(2, Duration::from_millis(1), Duration::from_millis(1));
        assert!(policy.should_retry(0));
        assert!(policy.should_retry(1));
        assert!(!policy.should_retry(2));
    }
}
