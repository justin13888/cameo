# Default recipe — show available commands
default:
    @just --list

# Format all code
fmt:
    cargo fmt --all

# Check formatting (CI mode)
fmt-check:
    cargo fmt --all -- --check

# Run clippy lints
clippy:
    cargo clippy --all-targets -- -D warnings

# Auto-fix clippy lints
clippy-fix:
    cargo clippy --fix --allow-dirty --allow-staged --all-targets -- -D warnings

# Build (default features)
build:
    cargo build

# Build with specific features
build-features features:
    cargo build --no-default-features --features "{{features}}"

# Run tests (default features)
test:
    cargo test

# Run tests with specific features
test-features features:
    cargo test --no-default-features --features "{{features}}"

# Run all tests including AniList
test-all:
    cargo test --features full

# Run live tests (requires TMDB_API_TOKEN in env or .env)
test-live:
    set -a; source .env; set +a && cargo test --features full,live-tests -- --test-threads=1

# Generate docs
doc:
    cargo doc --no-deps --open

# Check docs build (no open)
doc-check:
    cargo doc --no-deps --features full

# Run doc tests
doc-test:
    cargo test --doc --features full

# Pre-publish checks
publish-check:
    cargo clippy --all-targets --features full -- -D warnings
    cargo test --features full
    cargo doc --no-deps
    cargo publish --dry-run

# Regenerate TMDB OpenAPI spec
fetch-openapi:
    ./scripts/fetch-openapi.sh

# Install git hooks
setup:
    lefthook install
    cargo build

