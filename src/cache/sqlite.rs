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
    write_count: Arc<std::sync::atomic::AtomicU64>,
    /// Tracks total reads; used to trigger periodic expiry purges.
    read_count: Arc<std::sync::atomic::AtomicU64>,
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
        write_conn
            .execute_batch(CREATE_TABLE)
            .map_err(|e| CacheError::Backend(Box::new(e)))?;

        // Separate reader: in WAL mode this connection can read without
        // blocking the writer connection.
        let read_conn = Connection::open(path).map_err(|e| CacheError::Backend(Box::new(e)))?;

        Ok(Self {
            read_conn: Arc::new(Mutex::new(read_conn)),
            write_conn: Arc::new(Mutex::new(write_conn)),
            write_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            read_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        })
    }

    /// Create an in-memory SQLite cache (useful for testing).
    ///
    /// A single connection is used for both reads and writes because
    /// SQLite in-memory databases are not shared across connections.
    pub fn in_memory() -> Result<Self, CacheError> {
        let conn = Connection::open_in_memory().map_err(|e| CacheError::Backend(Box::new(e)))?;
        conn.execute_batch(CREATE_TABLE)
            .map_err(|e| CacheError::Backend(Box::new(e)))?;
        let conn = Arc::new(Mutex::new(conn));
        Ok(Self {
            // Both arcs point at the same mutex so reads and writes
            // serialise correctly with a single in-memory connection.
            read_conn: Arc::clone(&conn),
            write_conn: conn,
            write_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            read_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        })
    }

    /// Delete all expired cache entries immediately.
    ///
    /// Normally expiry is handled lazily on reads and periodically on writes.
    /// Call this to force immediate eviction of stale rows.
    pub async fn purge_expired(&self) -> Result<(), CacheError> {
        let conn = Arc::clone(&self.write_conn);
        let now = Self::now_secs();
        tokio::task::spawn_blocking(move || {
            let conn = conn
                .lock()
                .map_err(|e| CacheError::Backend(Box::from(format!("mutex poisoned: {e}"))))?;
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

    fn now_secs() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    /// Periodically purge expired rows (every ~100 writes).
    fn maybe_purge(conn: &Connection, count: u64) {
        if count != 0 && count.is_multiple_of(100) {
            let now = Self::now_secs();
            let _ = conn.execute(
                "DELETE FROM cache_entries WHERE expires_at < ?1",
                params![now as i64],
            );
        }
    }

    /// Trigger a best-effort expiry purge on the write connection every ~1000 reads.
    ///
    /// On read-heavy workloads the write-side purge (every 100 writes) may
    /// not fire often enough. This companion method ensures expired rows are
    /// eventually reclaimed even when writes are rare.
    fn maybe_purge_on_read(write_conn: Arc<Mutex<Connection>>, count: u64) {
        if count != 0 && count.is_multiple_of(1000) {
            let _purge_task = tokio::task::spawn_blocking(move || {
                if let Ok(conn) = write_conn.lock() {
                    let now = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs();
                    let _ = conn.execute(
                        "DELETE FROM cache_entries WHERE expires_at < ?1",
                        params![now as i64],
                    );
                }
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
        let now = Self::now_secs();

        let result = tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|e| {
                CacheError::Backend(Box::from(format!("mutex poisoned: {e}")))
            })?;
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
        let write_count = Arc::clone(&self.write_count);
        let key_type = key.key_type().to_string();
        let key_id = key.key_id();
        let json_str = serde_json::to_string(&value)?;
        let expires_at = Self::now_secs() + ttl.as_secs();

        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|e| {
                CacheError::Backend(Box::from(format!("mutex poisoned: {e}")))
            })?;
            conn.execute(
                "INSERT OR REPLACE INTO cache_entries (key_type, key_id, value, expires_at) VALUES (?1, ?2, ?3, ?4)",
                params![key_type, key_id, json_str, expires_at as i64],
            )
            .map_err(|e| CacheError::Backend(Box::new(e)))?;

            let count = write_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            Self::maybe_purge(&conn, count);

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
            let conn = conn
                .lock()
                .map_err(|e| CacheError::Backend(Box::from(format!("mutex poisoned: {e}"))))?;
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
            let conn = conn
                .lock()
                .map_err(|e| CacheError::Backend(Box::from(format!("mutex poisoned: {e}"))))?;
            conn.execute("DELETE FROM cache_entries", [])
                .map_err(|e| CacheError::Backend(Box::new(e)))?;
            Ok(())
        })
        .await
        .map_err(|e| CacheError::Backend(Box::from(format!("spawn_blocking failed: {e}"))))?
    }
}
