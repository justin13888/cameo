//! Repository automation for cameo.
//!
//! Currently exposes a single command, `codegen`, which regenerates the
//! vendored TMDB client from the committed OpenAPI 3.1 spec:
//!
//! ```text
//! cargo xtask codegen
//! ```
//!
//! It reads `openapi/tmdb-api.json`, runs spargen, and writes
//! `src/generated/tmdb.rs`. spargen is deterministic (same version + spec +
//! config gives byte-identical output), and the pinned version plus the
//! committed `xtask/Cargo.lock` make the output reproducible, which is what the
//! CI freshness check relies on.
//!
//! No post-generation patch is applied: spargen's generated `Client` carries
//! v3 `api_key` query auth natively (`with_v3_api_key`), and its `Debug`
//! redacts the registered credential.

use std::path::{Path, PathBuf};

/// The spec, relative to the repository root. Relative so the provenance
/// header spargen writes into the output does not depend on the checkout path.
const SPEC: &str = "openapi/tmdb-api.json";

/// The generated module, relative to the repository root.
const OUT: &str = "src/generated/tmdb.rs";

/// The repository root (the parent of the `xtask/` manifest directory).
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask/ always has a parent directory")
        .to_path_buf()
}

/// Regenerate `src/generated/tmdb.rs` from `openapi/tmdb-api.json`.
fn codegen() -> Result<(), Box<dyn std::error::Error>> {
    std::env::set_current_dir(repo_root())?;

    let build = spargen::Spec::new(SPEC)
        .build(OUT)
        // Not a build script: emit no `cargo:` directives and skip the
        // consumer-manifest audit, silently.
        .with_cargo(spargen::CargoIntegration::Off);
    let report = spargen::generate(&build);
    for warning in report.warnings() {
        eprintln!("{warning}");
    }
    let report = report.into_result().map_err(|report| report.to_string())?;
    eprintln!("spargen: {} -> {OUT}", report.outcome());
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    match std::env::args().nth(1).as_deref() {
        Some("codegen") => codegen(),
        Some(other) => {
            eprintln!("unknown command: {other}\nusage: cargo xtask codegen");
            std::process::exit(2);
        }
        None => {
            eprintln!("usage: cargo xtask codegen");
            std::process::exit(2);
        }
    }
}
