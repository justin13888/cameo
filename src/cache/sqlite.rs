//! SQLite-backed cache implementation.

use std::{
    path::Path,
    sync::{Arc, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use async_trait::async_trait;
use rusqlite::{Connection, OptionalExtension, params};

use super::{
    backend::{CacheBackend, CacheError},
    key::CacheKey,
};

const CREATE_TABLE: &str = "
CREATE TABLE IF NOT EXISTS cache_entries (
    key_type   TEXT    NOT NULL,
    key_id     TEXT    NOT NULL,
    value      TEXT    NOT NULL,
    expires_at INTEGER NOT NULL,
    PRIMARY KEY (key_type, key_id)
);
CREATE INDEX IF NOT EXISTS idx_expires ON cache_entries(expires_at);
";

/// On-disk cache schema/serialization version, stored in SQLite's
/// `PRAGMA user_version`. Bump this whenever the stored value shape changes
/// (e.g. a unified-model field change) so persisted entries from an older
/// cameo are flushed instead of deserialized as stale, wrong-shaped data.
const CACHE_SCHEMA_VERSION: i64 = 1;

/// Create the table and enforce the schema version: if the database's
/// `user_version` differs from [`CACHE_SCHEMA_VERSION`], every entry is flushed
/// (they may be an incompatible shape) and the version is stamped. A fresh
/// database starts at `user_version = 0`, so this also stamps new files.
fn init_schema(conn: &Connection) -> Result<(), CacheError> {
    conn.execute_batch(CREATE_TABLE)
        .map_err(CacheError::backend)?;
    let version: i64 = conn
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(CacheError::backend)?;
    if version != CACHE_SCHEMA_VERSION {
        conn.execute_batch("DELETE FROM cache_entries;")
            .map_err(CacheError::backend)?;
        conn.pragma_update(None, "user_version", CACHE_SCHEMA_VERSION)
            .map_err(CacheError::backend)?;
    }
    Ok(())
}

/// Source of the current unix time in seconds. Injectable so TTL behavior can
/// be tested deterministically without wall-clock sleeps.
type Clock = Arc<dyn Fn() -> u64 + Send + Sync>;

/// The largest number of entries kept on disk. Enforced on every write so the
/// database stays bounded even under quiet, long-running workloads where the
/// count-based expiry purge rarely fires.
const MAX_ENTRIES: i64 = 50_000;

fn system_clock() -> Clock {
    Arc::new(|| {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    })
}

/// SQLite-backed cache backend.
///
/// Uses a single `cache_entries` table with separate read and write
/// connections. For file-based databases, WAL journal mode is enabled so
/// that reads and background writes can proceed concurrently without
/// blocking each other: the read connection holds its own SQLite shared
/// lock while the write connection can proceed under WAL's snapshot
/// isolation.
///
/// For in-memory databases (e.g. in tests) a single connection is shared
/// because SQLite in-memory databases are not accessible from a second
/// connection. Reads and writes therefore still serialise through the same
/// mutex, which is correct and safe.
///
/// All rusqlite calls are dispatched to the blocking thread pool via
/// [`tokio::task::spawn_blocking`] to keep the async interface non-blocking.
#[derive(Clone)]
pub struct SqliteCache {
    /// Connection used exclusively for read (`SELECT`) queries.
    read_conn: Arc<Mutex<Connection>>,
    /// Connection used exclusively for write (`INSERT/DELETE`) queries.
    write_conn: Arc<Mutex<Connection>>,
    /// Tracks total reads; used to trigger periodic expiry purges.
    read_count: Arc<std::sync::atomic::AtomicU64>,
    /// Current-time source (injectable for deterministic tests).
    clock: Clock,
}

impl std::fmt::Debug for SqliteCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SqliteCache").finish_non_exhaustive()
    }
}

impl SqliteCache {
    /// Open or create a file-backed SQLite cache database.
    ///
    /// The write connection is switched to WAL journal mode and
    /// `synchronous=NORMAL`, which trades a small amount of durability
    /// (acceptable for a cache) for significantly reduced write latency.
    /// A separate read connection is opened so that concurrent reads do
    /// not contend with background writes.
    pub fn new(path: impl AsRef<Path>) -> Result<Self, CacheError> {
        let path = path.as_ref();

        // Writer: enable WAL for better concurrency and lower write latency.
        let write_conn = Connection::open(path).map_err(|e| CacheError::Backend(Box::new(e)))?;
        write_conn
            .execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")
            .map_err(|e| CacheError::Backend(Box::new(e)))?;
        init_schema(&write_conn)?;

        // Separate reader: in WAL mode this connection can read without
        // blocking the writer connection.
        let read_conn = Connection::open(path).map_err(|e| CacheError::Backend(Box::new(e)))?;

        Ok(Self {
            read_conn: Arc::new(Mutex::new(read_conn)),
            write_conn: Arc::new(Mutex::new(write_conn)),
            read_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            clock: system_clock(),
        })
    }

    /// Create an in-memory SQLite cache (useful for testing).
    ///
    /// A single connection is used for both reads and writes because
    /// SQLite in-memory databases are not shared across connections.
    pub fn in_memory() -> Result<Self, CacheError> {
        let conn = Connection::open_in_memory().map_err(|e| CacheError::Backend(Box::new(e)))?;
        init_schema(&conn)?;
        let conn = Arc::new(Mutex::new(conn));
        Ok(Self {
            // Both arcs point at the same mutex so reads and writes
            // serialise correctly with a single in-memory connection.
            read_conn: Arc::clone(&conn),
            write_conn: conn,
            read_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            clock: system_clock(),
        })
    }

    /// Delete all expired cache entries immediately.
    ///
    /// Normally expiry is handled lazily on reads and periodically on writes.
    /// Call this to force immediate eviction of stale rows.
    pub async fn purge_expired(&self) -> Result<(), CacheError> {
        let conn = Arc::clone(&self.write_conn);
        let now = self.now();
        tokio::task::spawn_blocking(move || {
            // Recover from a poisoned mutex rather than failing forever: cache
            // data is best-effort and a prior panic left it structurally intact.
            let conn = conn.lock().unwrap_or_else(|poison| poison.into_inner());
            conn.execute(
                "DELETE FROM cache_entries WHERE expires_at < ?1",
                rusqlite::params![now as i64],
            )
            .map_err(|e| CacheError::Backend(Box::new(e)))?;
            Ok(())
        })
        .await
        .map_err(|e| CacheError::Backend(Box::from(format!("spawn_blocking failed: {e}"))))?
    }

    /// Override the time source. Intended for deterministic TTL tests.
    #[cfg(test)]
    pub(crate) fn with_clock(mut self, clock: Clock) -> Self {
        self.clock = clock;
        self
    }

    fn now(&self) -> u64 {
        (self.clock)()
    }

    /// Reclaim expired rows and enforce the [`MAX_ENTRIES`] size cap. Called on
    /// every write so the file stays bounded regardless of workload.
    fn enforce_limits(conn: &Connection, now: u64) {
        let _ = conn.execute(
            "DELETE FROM cache_entries WHERE expires_at < ?1",
            params![now as i64],
        );
        let _ = conn.execute(
            "DELETE FROM cache_entries WHERE rowid NOT IN \
             (SELECT rowid FROM cache_entries ORDER BY expires_at DESC LIMIT ?1)",
            params![MAX_ENTRIES],
        );
    }

    /// Trigger a best-effort expiry purge on the write connection every ~1000 reads.
    ///
    /// On read-heavy workloads the write-side purge (every 100 writes) may
    /// not fire often enough. This companion method ensures expired rows are
    /// eventually reclaimed even when writes are rare.
    fn maybe_purge_on_read(write_conn: Arc<Mutex<Connection>>, count: u64) {
        if count != 0 && count.is_multiple_of(1000) {
            let _purge_task = tokio::task::spawn_blocking(move || {
                let conn = write_conn
                    .lock()
                    .unwrap_or_else(|poison| poison.into_inner());
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                let _ = conn.execute(
                    "DELETE FROM cache_entries WHERE expires_at < ?1",
                    params![now as i64],
                );
            });
        }
    }
}

#[async_trait]
impl CacheBackend for SqliteCache {
    #[tracing::instrument(skip(self, key), fields(key_type = key.key_type(), key_id = %key.key_id()))]
    async fn get(&self, key: &CacheKey) -> Result<Option<serde_json::Value>, CacheError> {
        let conn = Arc::clone(&self.read_conn);
        let write_conn = Arc::clone(&self.write_conn);
        let read_count = Arc::clone(&self.read_count);
        let key_type = key.key_type().to_string();
        let key_id = key.key_id();
        let now = self.now();

        let result = tokio::task::spawn_blocking(move || {
            let conn = conn.lock().unwrap_or_else(|poison| poison.into_inner());
            let result = conn.query_row(
                "SELECT value FROM cache_entries WHERE key_type = ?1 AND key_id = ?2 AND expires_at > ?3",
                params![key_type, key_id, now as i64],
                |row| row.get::<_, String>(0),
            ).optional();

            match result.map_err(|e| CacheError::Backend(Box::new(e)))? {
                Some(json_str) => {
                    let value = serde_json::from_str(&json_str)?;
                    Ok(Some(value))
                }
                None => Ok(None),
            }
        })
        .await
        .map_err(|e| CacheError::Backend(Box::from(format!("spawn_blocking failed: {e}"))))?;

        // Periodically evict expired rows even on read-heavy workloads.
        let count = read_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Self::maybe_purge_on_read(write_conn, count);

        result
    }

    #[tracing::instrument(skip(self, key, value), fields(key_type = key.key_type(), key_id = %key.key_id(), ttl_secs = ttl.as_secs()))]
    async fn set(
        &self,
        key: CacheKey,
        value: serde_json::Value,
        ttl: Duration,
    ) -> Result<(), CacheError> {
        let conn = Arc::clone(&self.write_conn);
        let key_type = key.key_type().to_string();
        let key_id = key.key_id();
        let json_str = serde_json::to_string(&value)?;
        let now = self.now();
        let expires_at = now + ttl.as_secs();

        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().unwrap_or_else(|poison| poison.into_inner());
            conn.execute(
                "INSERT OR REPLACE INTO cache_entries (key_type, key_id, value, expires_at) VALUES (?1, ?2, ?3, ?4)",
                params![key_type, key_id, json_str, expires_at as i64],
            )
            .map_err(|e| CacheError::Backend(Box::new(e)))?;

            // Bound the file: reclaim expired rows and trim to MAX_ENTRIES.
            Self::enforce_limits(&conn, now);

            Ok(())
        })
        .await
        .map_err(|e| CacheError::Backend(Box::from(format!("spawn_blocking failed: {e}"))))?
    }

    #[tracing::instrument(skip(self, key), fields(key_type = key.key_type(), key_id = %key.key_id()))]
    async fn invalidate(&self, key: &CacheKey) -> Result<(), CacheError> {
        let conn = Arc::clone(&self.write_conn);
        let key_type = key.key_type().to_string();
        let key_id = key.key_id();

        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().unwrap_or_else(|poison| poison.into_inner());
            conn.execute(
                "DELETE FROM cache_entries WHERE key_type = ?1 AND key_id = ?2",
                params![key_type, key_id],
            )
            .map_err(|e| CacheError::Backend(Box::new(e)))?;
            Ok(())
        })
        .await
        .map_err(|e| CacheError::Backend(Box::from(format!("spawn_blocking failed: {e}"))))?
    }

    async fn clear(&self) -> Result<(), CacheError> {
        let conn = Arc::clone(&self.write_conn);

        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().unwrap_or_else(|poison| poison.into_inner());
            conn.execute("DELETE FROM cache_entries", [])
                .map_err(|e| CacheError::Backend(Box::new(e)))?;
            Ok(())
        })
        .await
        .map_err(|e| CacheError::Backend(Box::from(format!("spawn_blocking failed: {e}"))))?
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::cache::{
        CacheBackend,
        key::{CacheKey, MediaType},
    };

    /// A schema-version mismatch flushes stale entries when the DB is reopened.
    #[tokio::test]
    async fn schema_version_bump_invalidates_old_entries() {
        let path = std::env::temp_dir().join(format!("cameo_schema_ver_{}.db", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let key = CacheKey::Detail {
            media_type: MediaType::Movie,
            provider_id: "tmdb:1".to_string(),
        };

        // Write an entry and confirm it is present.
        {
            let cache = SqliteCache::new(&path).unwrap();
            cache
                .set(
                    key.clone(),
                    serde_json::json!({ "v": 1 }),
                    Duration::from_secs(3600),
                )
                .await
                .unwrap();
            assert!(cache.get(&key).await.unwrap().is_some());
        }

        // Simulate a database written by an older/newer schema version.
        {
            let conn = Connection::open(&path).unwrap();
            conn.pragma_update(None, "user_version", 999i64).unwrap();
        }

        // Reopening detects the mismatch and flushes every entry.
        let cache = SqliteCache::new(&path).unwrap();
        assert!(cache.get(&key).await.unwrap().is_none());

        let _ = std::fs::remove_file(&path);
    }

    fn mock(now: &Arc<std::sync::atomic::AtomicU64>) -> Clock {
        let now = Arc::clone(now);
        Arc::new(move || now.load(std::sync::atomic::Ordering::Relaxed))
    }

    /// TTL expiry is exercised against an injected clock — no wall-clock sleep.
    #[tokio::test]
    async fn ttl_expiry_is_deterministic_with_mock_clock() {
        let now = Arc::new(std::sync::atomic::AtomicU64::new(1_000));
        let cache = SqliteCache::in_memory().unwrap().with_clock(mock(&now));
        let key = CacheKey::Detail {
            media_type: MediaType::Movie,
            provider_id: "tmdb:1".to_string(),
        };
        cache
            .set(
                key.clone(),
                serde_json::json!({ "v": 1 }),
                Duration::from_secs(60),
            )
            .await
            .unwrap();

        now.store(1_030, std::sync::atomic::Ordering::Relaxed); // within TTL
        assert!(cache.get(&key).await.unwrap().is_some());

        now.store(1_070, std::sync::atomic::Ordering::Relaxed); // past TTL
        assert!(cache.get(&key).await.unwrap().is_none());
    }

    /// Expired rows are reclaimed on write, so the file stays bounded even when
    /// nothing re-reads the stale entries.
    #[tokio::test]
    async fn expired_rows_are_reclaimed_on_write() {
        let now = Arc::new(std::sync::atomic::AtomicU64::new(1_000));
        let cache = SqliteCache::in_memory().unwrap().with_clock(mock(&now));
        let stale = CacheKey::Detail {
            media_type: MediaType::Movie,
            provider_id: "tmdb:1".to_string(),
        };
        cache
            .set(
                stale.clone(),
                serde_json::json!({}),
                Duration::from_secs(10),
            )
            .await
            .unwrap();

        now.store(2_000, std::sync::atomic::Ordering::Relaxed); // stale is now expired
        let fresh = CacheKey::Detail {
            media_type: MediaType::Movie,
            provider_id: "tmdb:2".to_string(),
        };
        cache
            .set(fresh, serde_json::json!({}), Duration::from_secs(3_600))
            .await
            .unwrap();

        // Rewind the clock: `stale` is only absent because the write reclaimed
        // it, not because a read-time filter hid it.
        now.store(1_000, std::sync::atomic::Ordering::Relaxed);
        assert!(cache.get(&stale).await.unwrap().is_none());
    }
}
