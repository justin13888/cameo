//! Repository automation for cameo.
//!
//! Currently exposes a single command, `codegen`, which regenerates the
//! vendored TMDB client from the committed OpenAPI spec:
//!
//! ```text
//! cargo xtask codegen
//! ```
//!
//! It reads `openapi/tmdb-api.json`, runs progenitor, formats the result with
//! prettyplease, and writes `src/generated/tmdb.rs`. Pinned dependencies plus
//! the committed `xtask/Cargo.lock` make the output reproducible, which is what
//! the CI freshness check relies on.

use std::path::{Path, PathBuf};

/// The repository root (the parent of the `xtask/` manifest directory).
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask/ always has a parent directory")
        .to_path_buf()
}

/// Regenerate `src/generated/tmdb.rs` from `openapi/tmdb-api.json`.
fn codegen() -> Result<(), Box<dyn std::error::Error>> {
    let root = repo_root();
    let spec_path = root.join("openapi").join("tmdb-api.json");
    let out_path = root.join("src").join("generated").join("tmdb.rs");

    let file = std::fs::File::open(&spec_path)
        .map_err(|e| format!("failed to open {}: {e}", spec_path.display()))?;
    let spec = serde_json::from_reader(file)?;

    let mut generator = progenitor::Generator::default();
    let tokens = generator.generate_tokens(&spec)?;
    let ast = syn::parse2(tokens)?;
    let content = prettyplease::unparse(&ast);

    std::fs::write(&out_path, &content)?;
    eprintln!("wrote {} ({} bytes)", out_path.display(), content.len());
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
