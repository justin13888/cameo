# Contributing to cameo

Thanks for your interest! This guide covers the workflows that aren't obvious
from the code.

## Getting started

```sh
cargo test --all-features        # run the suite
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --all
```

A [lefthook](https://github.com/evilmartians/lefthook) pre-push hook runs
formatting, clippy, and the tests across the feature-flag combinations; install
it with `lefthook install`. Use [conventional commits](https://www.conventionalcommits.org/)
(`type: description`).

## Toolchain and MSRV

The crate's **support target is stable Rust `1.88`** (the MSRV — see
`Cargo.toml` `rust-version` and [docs/STABILITY.md](docs/STABILITY.md)); the
`msrv` CI job builds and tests on it. `rust-toolchain.toml` pins a specific
**nightly only** because `rustfmt.toml` uses nightly-only formatting options
(import grouping) — that pinned nightly makes `cargo fmt` reproducible. Nothing
else requires nightly; don't introduce nightly language features.

## Regenerating the TMDB client

The TMDB low-level client under `src/generated/` is **vendored** — generated
from `openapi/tmdb-api.json` and committed so builds are hermetic. Do not edit
it by hand. To regenerate after a spec change:

```sh
./scripts/fetch-openapi.sh        # refresh + patch the vendored OpenAPI spec
cargo xtask codegen               # regenerate src/generated/tmdb.rs (+ v3-auth patch)
```

`cargo xtask codegen` is deterministic (pinned via `xtask/Cargo.lock`); the
`codegen-freshness` CI job fails if the committed output drifts from a fresh
run, so always commit the regenerated file.

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
