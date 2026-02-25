//! Cache backend trait and error type.

use std::time::Duration;

use async_trait::async_trait;

use super::key::CacheKey;

/// Error type for cache operations.
#[derive(Debug, thiserror::Error)]
pub enum CacheError {
    /// Serialization or deserialization failed.
    #[error("cache serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    /// The underlying backend encountered an error.
    #[error("cache backend error: {0}")]
    Backend(Box<dyn std::error::Error + Send + Sync>),
}

/// A pluggable cache backend.
///
/// Values are stored and retrieved as [`serde_json::Value`] so the trait is
/// object-safe. Callers handle typed serialization/deserialization.
#[async_trait]
pub trait CacheBackend: Send + Sync + 'static {
    /// Retrieve a cached value by key. Returns `None` if absent or expired.
    async fn get(&self, key: &CacheKey) -> Result<Option<serde_json::Value>, CacheError>;
    /// Store a value with the given TTL.
    async fn set(
        &self,
        key: CacheKey,
        value: serde_json::Value,
        ttl: Duration,
    ) -> Result<(), CacheError>;
    /// Remove a specific entry.
    async fn invalidate(&self, key: &CacheKey) -> Result<(), CacheError>;
    /// Remove all entries.
    async fn clear(&self) -> Result<(), CacheError>;
}
