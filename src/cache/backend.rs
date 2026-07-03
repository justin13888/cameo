//! Cache backend trait and error type — the pluggable persistence contract.

use std::time::Duration;

use async_trait::async_trait;

use super::key::CacheKey;

/// Error returned by [`CacheBackend`] operations.
///
/// This is the **v1 error contract**: it is `#[non_exhaustive]` (new kinds may
/// be added without a breaking release) and callers can match on the kind —
/// serialization problems are distinguished from backend/store failures. The
/// [`Backend`](CacheError::Backend) variant carries the underlying error as its
/// source for logging or downcasting.
///
/// Cache errors are non-fatal on the request path: the facade logs them and
/// falls back to a live fetch, so a failing cache never breaks a call.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum CacheError {
    /// The value could not be serialized to or deserialized from its stored form.
    #[error("cache serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    /// The underlying store failed (I/O, connection, corruption, …).
    #[error("cache backend error: {0}")]
    Backend(Box<dyn std::error::Error + Send + Sync>),
}

impl CacheError {
    /// Wrap an arbitrary store error as [`CacheError::Backend`].
    pub fn backend<E: std::error::Error + Send + Sync + 'static>(error: E) -> Self {
        CacheError::Backend(Box::new(error))
    }
}

/// A pluggable cache backend.
///
/// # v1 contract
///
/// - **Value representation** — values are opaque [`serde_json::Value`]s. The
///   facade owns typed (de)serialization; a backend treats the value as a blob
///   and may store it however it likes (JSON text, `serde_cbor`, etc.). This is
///   a deliberate choice: `Value` is serde-native, self-describing, and keeps
///   the trait object-safe without a generic parameter.
/// - **Keys** — a [`CacheKey`] identifies an entry. Derive a stable string from
///   [`CacheKey::key_type`] + [`CacheKey::key_id`]; both already embed the
///   provider/version namespace, so distinct providers and schema versions
///   never collide.
/// - **TTL** — [`set`](CacheBackend::set) is given a time-to-live; expired
///   entries must be reported as `None` by [`get`](CacheBackend::get).
/// - **Errors** — return [`CacheError`]; they are logged and treated as a miss,
///   never surfaced to the caller.
///
/// # Openness
///
/// The trait is **open**: any `Send + Sync + 'static` type may implement it to
/// plug in Redis, Memcached, an in-memory map, etc. Because it uses
/// `#[async_trait]`, implementers can reuse cameo's re-export
/// (`#[cameo::async_trait::async_trait]`) instead of adding their own
/// dependency.
///
/// ```
/// use std::collections::HashMap;
/// use std::sync::Mutex;
/// use std::time::{Duration, Instant};
///
/// use cameo::async_trait::async_trait;
/// use cameo::{CacheBackend, CacheError, CacheKey};
///
/// #[derive(Default)]
/// struct MemoryCache {
///     entries: Mutex<HashMap<String, (serde_json::Value, Instant)>>,
/// }
///
/// impl MemoryCache {
///     fn slot(key: &CacheKey) -> String {
///         format!("{}:{}", key.key_type(), key.key_id())
///     }
/// }
///
/// #[async_trait]
/// impl CacheBackend for MemoryCache {
///     async fn get(&self, key: &CacheKey) -> Result<Option<serde_json::Value>, CacheError> {
///         let entries = self.entries.lock().unwrap();
///         Ok(entries
///             .get(&Self::slot(key))
///             .filter(|(_, expires)| Instant::now() < *expires)
///             .map(|(value, _)| value.clone()))
///     }
///     async fn set(&self, key: CacheKey, value: serde_json::Value, ttl: Duration) -> Result<(), CacheError> {
///         self.entries.lock().unwrap().insert(Self::slot(&key), (value, Instant::now() + ttl));
///         Ok(())
///     }
///     async fn invalidate(&self, key: &CacheKey) -> Result<(), CacheError> {
///         self.entries.lock().unwrap().remove(&Self::slot(key));
///         Ok(())
///     }
///     async fn clear(&self) -> Result<(), CacheError> {
///         self.entries.lock().unwrap().clear();
///         Ok(())
///     }
/// }
///
/// // The custom backend plugs straight into the object-safe trait.
/// let _backend: std::sync::Arc<dyn CacheBackend> = std::sync::Arc::new(MemoryCache::default());
/// ```
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
