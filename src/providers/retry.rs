//! Retry policy for transient provider failures.
//!
//! Retries 429/5xx and connect/transport errors with jittered exponential
//! backoff, honoring a server-supplied `Retry-After` when present. The retry
//! loops live in each provider (they classify their own errors) and re-acquire
//! the rate limiter before every attempt, so retries cannot stampede.

use std::time::Duration;

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
