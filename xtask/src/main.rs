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
    let content = patch_v3_auth(&content)?;

    std::fs::write(&out_path, &content)?;
    eprintln!("wrote {} ({} bytes)", out_path.display(), content.len());
    Ok(())
}

/// Deterministic post-generation patch enabling TMDB v3 `?api_key=` auth.
///
/// Progenitor's generated `Client` has no place to carry a global query param,
/// and reqwest has no default-query support, so v3 API-key auth cannot be added
/// from outside the generated code. This patch gives the client an optional
/// `auth_query` and an implemented `pre` request hook that appends it to every
/// request. It is applied here (rather than hand-editing the vendored file) so
/// that a fresh `cargo xtask codegen` reproduces the exact committed output and
/// the codegen-freshness CI stays green.
///
/// It also replaces the generated `Client`'s derived `Debug` with a manual impl
/// that prints `auth_query`'s value as `<redacted>`, so the API key cannot leak
/// through `{:?}` formatting.
fn patch_v3_auth(content: &str) -> Result<String, Box<dyn std::error::Error>> {
    let content = content.replacen(
        "    pub(crate) client: reqwest::Client,\n}",
        "    pub(crate) client: reqwest::Client,\n    pub(crate) auth_query: \
         ::std::option::Option<(String, String)>,\n}",
        1,
    );
    let content = content.replacen(
        "            baseurl: baseurl.to_string(),\n            client,\n        }",
        "            baseurl: baseurl.to_string(),\n            client,\n            \
         auth_query: None,\n        }",
        1,
    );
    let hook = "impl ClientHooks<()> for &Client {\n    \
                async fn pre<E>(\n        &self,\n        \
                request: &mut reqwest::Request,\n        \
                _info: &OperationInfo,\n    ) -> ::std::result::Result<(), Error<E>> {\n        \
                if let Some((key, value)) = &self.auth_query {\n            \
                request.url_mut().query_pairs_mut().append_pair(key, value);\n        }\n        \
                Ok(())\n    }\n}";
    let content = content.replacen("impl ClientHooks<()> for &Client {}", hook, 1);

    // The derived `Debug` would print the v3 API key held in `auth_query`;
    // swap it for a manual impl that keeps the parameter name but redacts the
    // value.
    let content = content.replacen(
        "#[derive(Clone, Debug)]\n/**Client for tmdb-api",
        "#[derive(Clone)]\n/**Client for tmdb-api",
        1,
    );
    let debug_impl = "impl ::std::fmt::Debug for Client {\n    \
                      fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {\n        \
                      f.debug_struct(\"Client\")\n            \
                      .field(\"baseurl\", &self.baseurl)\n            \
                      .field(\"client\", &self.client)\n            \
                      .field(\n                \"auth_query\",\n                \
                      &self.auth_query.as_ref().map(|(key, _)| (key.as_str(), \"<redacted>\")),\n            \
                      )\n            \
                      .finish()\n    }\n}\nimpl ClientInfo<()> for Client {";
    let content = content.replacen("impl ClientInfo<()> for Client {", debug_impl, 1);

    if !content.contains("auth_query: ::std::option::Option<(String, String)>")
        || !content.contains("query_pairs_mut().append_pair")
        || !content.contains("#[derive(Clone)]\n/**Client for tmdb-api")
        || !content.contains("impl ::std::fmt::Debug for Client {")
    {
        return Err(
            "v3 auth patch did not apply — the generated Client layout changed; \
             update patch_v3_auth in xtask"
                .into(),
        );
    }
    Ok(content)
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
