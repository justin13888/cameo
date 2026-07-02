//! A real per-provider request-rate limiter.
//!
//! Replaces the old concurrency semaphore, which only bounded *simultaneous*
//! in-flight requests (permits were released as soon as a response returned, so
//! fast responses trivially exceeded a provider's real throughput limit). This
//! is a token bucket: it bounds the *sustained request rate* and paces bursts.
//!
//! Time is read through [`tokio::time`], so tests can drive it deterministically
//! with `#[tokio::test(start_paused = true)]` — no wall-clock sleeps.

use std::{sync::Arc, time::Duration};

use tokio::sync::Mutex;

/// A client-side request-rate policy.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateLimit {
    /// No client-side rate limiting.
    Unlimited,
    /// At most `max_requests` requests per `per` window.
    ///
    /// Modeled as a token bucket with capacity `max_requests` that refills
    /// `max_requests` tokens every `per` — so a burst of up to `max_requests`
    /// is allowed, after which requests are paced at `per / max_requests` apart.
    Limited {
        /// Maximum requests per window (also the burst capacity).
        max_requests: u32,
        /// The window over which `max_requests` are allowed.
        per: Duration,
    },
}

impl RateLimit {
    /// A `Limited` policy of `max_requests` per `per`.
    pub const fn per(max_requests: u32, per: Duration) -> Self {
        RateLimit::Limited { max_requests, per }
    }

    /// Reject nonsensical configuration (zero requests or a zero window) at
    /// construction time, so misconfiguration is an error rather than a runtime
    /// deadlock.
    pub(crate) fn validate(&self) -> Result<(), String> {
        if let RateLimit::Limited { max_requests, per } = self {
            if *max_requests == 0 {
                return Err("rate limit `max_requests` must be greater than 0".to_string());
            }
            if per.is_zero() {
                return Err("rate limit `per` window must be greater than 0".to_string());
            }
        }
        Ok(())
    }
}

/// A token-bucket rate limiter, cheap to clone (shares one bucket).
#[derive(Debug, Clone)]
pub(crate) struct RateLimiter {
    /// `None` for [`RateLimit::Unlimited`].
    bucket: Option<Arc<Mutex<Bucket>>>,
    /// Time to accrue one token (`per / max_requests`).
    interval: Duration,
    /// Bucket capacity (`max_requests`).
    capacity: f64,
}

#[derive(Debug)]
struct Bucket {
    tokens: f64,
    /// `None` until the first acquire, so construction needs no runtime clock.
    last: Option<tokio::time::Instant>,
}

impl RateLimiter {
    /// Build a limiter for the given policy. Assumes `limit` has been validated.
    pub(crate) fn new(limit: RateLimit) -> Self {
        match limit {
            RateLimit::Unlimited => Self {
                bucket: None,
                interval: Duration::ZERO,
                capacity: 0.0,
            },
            RateLimit::Limited { max_requests, per } => {
                let capacity = f64::from(max_requests.max(1));
                Self {
                    bucket: Some(Arc::new(Mutex::new(Bucket {
                        tokens: capacity, // start full: allow an initial burst
                        last: None,
                    }))),
                    interval: per / max_requests.max(1),
                    capacity,
                }
            }
        }
    }

    /// Wait until a request may proceed, consuming one token.
    ///
    /// Returns immediately when a token is available (or when unlimited);
    /// otherwise sleeps until the next token refills.
    pub(crate) async fn acquire(&self) {
        let Some(bucket) = &self.bucket else {
            return;
        };
        let refill_per_sec = 1.0 / self.interval.as_secs_f64();

        let mut b = bucket.lock().await;
        let now = tokio::time::Instant::now();
        if let Some(last) = b.last {
            let elapsed = now.saturating_duration_since(last).as_secs_f64();
            b.tokens = (b.tokens + elapsed * refill_per_sec).min(self.capacity);
        }
        b.last = Some(now);

        if b.tokens >= 1.0 {
            b.tokens -= 1.0;
            return;
        }

        // Not enough: wait for the deficit to refill. Holding the lock across
        // the sleep serializes waiters, so they are released at the bucket rate.
        let deficit = 1.0 - b.tokens;
        let until = now + self.interval.mul_f64(deficit);
        b.tokens = 0.0;
        b.last = Some(until);
        tokio::time::sleep_until(until).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_rejects_zero_values() {
        assert!(
            RateLimit::Limited {
                max_requests: 0,
                per: Duration::from_secs(1)
            }
            .validate()
            .is_err()
        );
        assert!(
            RateLimit::Limited {
                max_requests: 5,
                per: Duration::ZERO
            }
            .validate()
            .is_err()
        );
        assert!(RateLimit::per(5, Duration::from_secs(1)).validate().is_ok());
        assert!(RateLimit::Unlimited.validate().is_ok());
    }

    /// With mock time, prove the sustained rate is bounded: 15 requests through
    /// a 5/second limiter (capacity 5) take at least ~2s of virtual time (the
    /// 5-request burst is instant, the remaining 10 are paced at 200ms each).
    #[tokio::test(start_paused = true)]
    async fn sustained_rate_is_bounded() {
        let limiter = RateLimiter::new(RateLimit::per(5, Duration::from_secs(1)));
        let start = tokio::time::Instant::now();
        for _ in 0..15 {
            limiter.acquire().await;
        }
        let elapsed = start.elapsed();
        assert!(
            elapsed >= Duration::from_millis(1900),
            "15 requests at 5/s took only {elapsed:?}; rate is not bounded"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn unlimited_never_waits() {
        let limiter = RateLimiter::new(RateLimit::Unlimited);
        let start = tokio::time::Instant::now();
        for _ in 0..1000 {
            limiter.acquire().await;
        }
        assert_eq!(start.elapsed(), Duration::ZERO);
    }

    /// The initial burst is capped at capacity, not unbounded.
    #[tokio::test(start_paused = true)]
    async fn initial_burst_capped_at_capacity() {
        let limiter = RateLimiter::new(RateLimit::per(3, Duration::from_secs(1)));
        let start = tokio::time::Instant::now();
        for _ in 0..3 {
            limiter.acquire().await;
        }
        // First 3 are the burst — instant.
        assert_eq!(start.elapsed(), Duration::ZERO);
        // The 4th must wait ~333ms for a refill.
        limiter.acquire().await;
        assert!(start.elapsed() >= Duration::from_millis(300));
    }
}
