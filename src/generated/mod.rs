/// Auto-generated TMDB API client from the OpenAPI spec via progenitor.
///
/// This module re-exports the generated client, types, and error types.
///
/// The generated code is **vendored** in `src/generated/tmdb.rs` and committed
/// to the repository — it is not regenerated during a normal `cargo build`. To
/// regenerate it after changing `openapi/tmdb-api.json`, run `cargo xtask
/// codegen` (see `DEVELOPMENT.md`). Do not edit `tmdb.rs` by hand.
#[allow(
    clippy::all,
    clippy::pedantic,
    missing_docs,
    unused_imports,
    unreachable_code,
    unreachable_patterns
)]
pub mod tmdb {
    include!("tmdb.rs");
}
