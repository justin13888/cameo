//! Helpers shared by the unit tests under `src/`.

use std::path::{Path, PathBuf};

/// A directory under the temporary directory that belongs to one test, removed
/// with everything in it (a SQLite database's `-wal`/`-shm` files included)
/// when dropped, so a panicking test leaves nothing behind.
///
/// The name carries the test's `name` and the process id: unique per test
/// under `cargo test` (one process, distinct names) and under nextest (one
/// process per test).
pub(crate) struct TempDir(PathBuf);

impl TempDir {
    /// Create an empty `cameo-{name}-{pid}` directory, clearing any leftover.
    pub(crate) fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("cameo-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create the test's temporary directory");
        Self(dir)
    }

    /// The directory's path.
    pub(crate) fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
