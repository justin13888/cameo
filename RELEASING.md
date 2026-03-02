# Release Process

This document describes the end-to-end release process for `cameo`.

## Prerequisites

- Push rights to the `master` branch and ability to create tags on this repo
- `CARGO_REGISTRY_TOKEN` secret configured in the GitHub repo settings (one-time setup; see CD workflow)
- Authenticated crates.io account with ownership of the `cameo` crate (first publish only)

## Pre-Release Checklist

- [ ] All tests pass locally: `cargo test --all-features`
- [ ] `cargo publish --dry-run` succeeds (no dirty tree)
- [ ] `cargo doc --no-deps` builds without errors or warnings
- [ ] Clippy passes: `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] CI is green on `master` (check GitHub Actions)
- [ ] `CHANGELOG.md` `[Unreleased]` section is populated with all changes since last release
- [ ] Version in `Cargo.toml` matches the intended release tag (e.g. `0.2.0`)

## Release Steps

1. Update the `version` field in `Cargo.toml` (e.g. `0.1.0` → `0.2.0`)
2. Move `CHANGELOG.md` `[Unreleased]` entries to a new dated section `[X.Y.Z] - YYYY-MM-DD`
3. Commit: `git commit -am 'chore: release v0.2.0'`
4. Tag: `git tag -a v0.2.0 -m 'Release v0.2.0'`
5. Push: `git push origin master && git push origin v0.2.0`
6. GitHub Actions release workflow triggers automatically on the tag push
7. Verify the release appears at <https://crates.io/crates/cameo>

## Manual Publish (if CD is unavailable)

```sh
git checkout v0.2.0
cargo publish
```

## Post-Release

- Create a GitHub Release from the tag, using the relevant `CHANGELOG.md` section as release notes
- Verify that <https://docs.rs/cameo/latest> resolves correctly (may take a few minutes to build)

