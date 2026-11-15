use std::time::Duration;

use super::*;
use crate::cache::{
    CacheBackend,
    key::{CacheKey, MediaType},
};

/// A blocking task that panics surfaces as `Backend` carrying the
/// `JoinError` itself, not a stringified copy of it.
#[tokio::test]
async fn run_blocking_keeps_the_join_error() {
    let err = run_blocking::<(), _>(|| panic!("boom"))
        .await
        .expect_err("a panicking task is an error");
    let CacheError::Backend(source) = err else {
        panic!("expected CacheError::Backend, got {err:?}");
    };
    let join = source
        .downcast_ref::<tokio::task::JoinError>()
        .expect("the boxed source is the JoinError");
    assert!(join.is_panic());
}

/// Work that succeeds or fails on its own passes its result through.
#[tokio::test]
async fn run_blocking_passes_the_work_result_through() {
    assert_eq!(run_blocking(|| Ok(7)).await.unwrap(), 7);
    let err = run_blocking::<(), _>(|| Err(CacheError::backend(std::fmt::Error)))
        .await
        .expect_err("the work's own error is returned");
    let CacheError::Backend(source) = err else {
        panic!("expected CacheError::Backend, got {err:?}");
    };
    assert!(source.downcast_ref::<std::fmt::Error>().is_some());
}

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

fn row_count(cache: &SqliteCache) -> i64 {
    let conn = cache.write_conn.lock().unwrap();
    conn.query_row("SELECT COUNT(*) FROM cache_entries", [], |row| row.get(0))
        .unwrap()
}

fn detail(id: u32) -> CacheKey {
    CacheKey::Detail {
        media_type: MediaType::Movie,
        provider_id: format!("tmdb:{id}"),
    }
}

/// `purge_expired` deletes exactly the rows whose expiry is before the
/// injected clock, and keeps rows that expire now or later.
#[tokio::test]
async fn purge_expired_deletes_only_expired_rows() {
    let now = Arc::new(std::sync::atomic::AtomicU64::new(1_000));
    let cache = SqliteCache::in_memory().unwrap().with_clock(mock(&now));
    for (id, ttl) in [(1, 10), (2, 50), (3, 500)] {
        cache
            .set(
                detail(id),
                serde_json::json!({ "id": id }),
                Duration::from_secs(ttl),
            )
            .await
            .unwrap();
    }
    assert_eq!(row_count(&cache), 3);

    // Rows expire at 1_010, 1_050 and 1_500; at 1_050 only the first is
    // strictly past its expiry.
    now.store(1_050, std::sync::atomic::Ordering::Relaxed);
    cache.purge_expired().await.unwrap();
    assert_eq!(row_count(&cache), 2);

    // Rewind so the read filter hides nothing: absence is the purge's doing.
    now.store(0, std::sync::atomic::Ordering::Relaxed);
    assert!(cache.get(&detail(1)).await.unwrap().is_none());
    assert!(cache.get(&detail(2)).await.unwrap().is_some());
    assert!(cache.get(&detail(3)).await.unwrap().is_some());

    now.store(10_000, std::sync::atomic::Ordering::Relaxed);
    cache.purge_expired().await.unwrap();
    assert_eq!(row_count(&cache), 0);
}

/// A write trims the table to `MAX_ENTRIES`, evicting the rows that expire
/// soonest.
#[tokio::test]
async fn write_enforces_max_entries_by_evicting_soonest_expiring() {
    let now = Arc::new(std::sync::atomic::AtomicU64::new(1_000));
    let cache = SqliteCache::in_memory().unwrap().with_clock(mock(&now));
    // Fill the table to the cap directly; going through `set` would run
    // the trim on every one of the 50 000 writes.
    {
        let mut conn = cache.write_conn.lock().unwrap();
        let tx = conn.transaction().unwrap();
        {
            let mut insert = tx
                .prepare(
                    "INSERT INTO cache_entries (key_type, key_id, value, expires_at) \
                         VALUES ('detail', ?1, '{}', ?2)",
                )
                .unwrap();
            for i in 0..MAX_ENTRIES {
                insert
                    .execute(params![format!("movie:bulk:{i}"), 100_000 + i])
                    .unwrap();
            }
        }
        tx.commit().unwrap();
    }
    assert_eq!(row_count(&cache), MAX_ENTRIES);

    cache
        .set(
            detail(1),
            serde_json::json!({ "fresh": true }),
            Duration::from_secs(1_000_000),
        )
        .await
        .unwrap();

    assert_eq!(row_count(&cache), MAX_ENTRIES);
    assert!(cache.get(&detail(1)).await.unwrap().is_some());
    let conn = cache.write_conn.lock().unwrap();
    let soonest: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM cache_entries WHERE key_id = 'movie:bulk:0'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let next: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM cache_entries WHERE key_id = 'movie:bulk:1'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!((soonest, next), (0, 1));
}
