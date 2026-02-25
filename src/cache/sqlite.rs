//! SQLite-backed cache implementation.

use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use rusqlite::{Connection, OptionalExtension, params};

use super::backend::{CacheBackend, CacheError};
use super::key::CacheKey;

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
/// Uses a single `cache_entries` table. All rusqlite calls are run on a
/// blocking thread pool via [`tokio::task::spawn_blocking`] to keep the async
/// interface non-blocking.
#[derive(Clone)]
pub struct SqliteCache {
    conn: Arc<Mutex<Connection>>,
    write_count: Arc<std::sync::atomic::AtomicU64>,
}

impl SqliteCache {
    /// Open or create a SQLite cache database at the given path.
    pub fn new(path: impl AsRef<Path>) -> Result<Self, CacheError> {
        let conn = Connection::open(path)
            .map_err(|e| CacheError::Backend(Box::new(e)))?;
        Self::init(conn)
    }

    /// Create an in-memory SQLite cache (useful for testing).
    pub fn in_memory() -> Result<Self, CacheError> {
        let conn =
            Connection::open_in_memory().map_err(|e| CacheError::Backend(Box::new(e)))?;
        Self::init(conn)
    }

    fn init(conn: Connection) -> Result<Self, CacheError> {
        conn.execute_batch(CREATE_TABLE)
            .map_err(|e| CacheError::Backend(Box::new(e)))?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            write_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        })
    }

    fn now_secs() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    /// Periodically purge expired rows (every ~100 writes).
    fn maybe_purge(conn: &Connection, count: u64) {
        if count % 100 == 0 {
            let now = Self::now_secs();
            let _ = conn.execute(
                "DELETE FROM cache_entries WHERE expires_at < ?1",
                params![now as i64],
            );
        }
    }
}

#[async_trait]
impl CacheBackend for SqliteCache {
    async fn get(&self, key: &CacheKey) -> Result<Option<serde_json::Value>, CacheError> {
        let conn = Arc::clone(&self.conn);
        let key_type = key.key_type().to_string();
        let key_id = key.key_id();
        let now = Self::now_secs();

        tokio::task::spawn_blocking(move || {
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
        .map_err(|e| CacheError::Backend(Box::from(format!("spawn_blocking failed: {e}"))))?
    }

    async fn set(
        &self,
        key: CacheKey,
        value: serde_json::Value,
        ttl: Duration,
    ) -> Result<(), CacheError> {
        let conn = Arc::clone(&self.conn);
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

    async fn invalidate(&self, key: &CacheKey) -> Result<(), CacheError> {
        let conn = Arc::clone(&self.conn);
        let key_type = key.key_type().to_string();
        let key_id = key.key_id();

        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|e| {
                CacheError::Backend(Box::from(format!("mutex poisoned: {e}")))
            })?;
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
        let conn = Arc::clone(&self.conn);

        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|e| {
                CacheError::Backend(Box::from(format!("mutex poisoned: {e}")))
            })?;
            conn.execute("DELETE FROM cache_entries", [])
                .map_err(|e| CacheError::Backend(Box::new(e)))?;
            Ok(())
        })
        .await
        .map_err(|e| CacheError::Backend(Box::from(format!("spawn_blocking failed: {e}"))))?
    }
}
