# Contributing to cameo

Thanks for your interest! This guide covers the workflows that aren't obvious
from the code.

## Getting started

```sh
cargo install --locked cargo-nextest   # once; the test runner (https://nexte.st)
cargo nextest run --features full      # run the suite (or `just test`)
cargo test --doc --features full       # doctests (nextest does not run them)
cargo clippy --all-targets --features full -- -D warnings
cargo +nightly fmt --all                # rustfmt.toml needs nightly (`just fmt`)
```

`cargo nextest run --all-features` is safe: the live tests that `live-tests`
compiles in hit the real TMDB and AniList APIs and need a `TMDB_API_TOKEN`, so
`.config/nextest.toml` filters them out unless you run `just test-live`. Plain
`cargo test --all-features` has no such filter and runs them.

A [lefthook](https://github.com/evilmartians/lefthook) pre-push hook runs
formatting, clippy, and the tests and doctests across the feature-flag
combinations; install
it with `lefthook install`. Use [conventional commits](https://www.conventionalcommits.org/)
(`type: description`).

## Toolchain and MSRV

The crate's **support target is stable Rust at its MSRV**, declared once as
`rust-version` in `Cargo.toml` (policy in [docs/STABILITY.md](docs/STABILITY.md));
the `msrv` CI job reads it from there and runs `cargo +<msrv> check
--all-targets` (it does not run the tests), once against the committed
`Cargo.lock` and once against a fresh MSRV-aware resolution, which is what a
downstream user gets. Day to day, `rust-toolchain.toml` builds on the latest
**stable**; breakage a new stable brings is fixed when it lands. Only
formatting needs nightly, because `rustfmt.toml` uses nightly-only options
(import grouping): run `cargo +nightly fmt --all` (`just fmt`). Nothing else
requires nightly; don't introduce nightly language features.

## Regenerating the TMDB client

The TMDB low-level client under `src/generated/` is **vendored** — generated
from `openapi/tmdb-api.json` and committed so builds are hermetic. Do not edit
it by hand; regenerate it with `cargo xtask codegen` and commit the result (the
`codegen-freshness` CI job fails on drift). The full workflow, including
refreshing the spec, is in [DEVELOPMENT.md](DEVELOPMENT.md#regenerating-the-tmdb-openapi-spec).

## Bump the cache version on model changes

The persistent SQLite cache stores serialized unified models. **Any change to a
`Unified*` model's serialized shape must bump `CACHE_SCHEMA_VERSION` in
`src/cache/sqlite.rs`.** That stamps `PRAGMA user_version` and flushes stale
entries on upgrade, so users never deserialize old-shaped JSON as if it were
current.

## Adding a provider

Providers implement the object-safe capability traits and register through
`CameoClient::builder().register_*_provider(...)` — no facade edits required.
See the end-to-end guide in [docs/providers.md](docs/providers.md) (with a
compiling public-API-only example in `tests/integration/registry_test.rs`) and
the design rationale in `docs/adr/0001-provider-abstraction.md`.
