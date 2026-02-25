//! Caching layer for the cameo SDK.
//!
//! Provides a pluggable [`CacheBackend`] trait and a default [`SqliteCache`]
//! implementation. Use [`CacheTtlConfig`] to control how long different
//! response types are cached.

pub mod backend;
pub mod key;
pub mod sqlite;

pub use backend::{CacheBackend, CacheError};
pub use key::{CacheKey, MediaType};
pub use sqlite::SqliteCache;

use std::time::Duration;

/// TTL configuration for the cache layer.
///
/// Different response types have different staleness tolerances.
#[derive(Debug, Clone)]
pub struct CacheTtlConfig {
    /// TTL for detail responses (e.g. full movie/TV/person details).
    pub details: Duration,
    /// TTL for search result pages.
    pub search: Duration,
    /// TTL for discovery/listing result pages.
    pub discovery: Duration,
    /// TTL for individual items indexed from list results.
    pub items: Duration,
}

impl Default for CacheTtlConfig {
    fn default() -> Self {
        Self {
            details: Duration::from_secs(24 * 3600),    // 24 hours
            search: Duration::from_secs(3600),           // 1 hour
            discovery: Duration::from_secs(15 * 60),     // 15 minutes
            items: Duration::from_secs(6 * 3600),        // 6 hours
        }
    }
}
