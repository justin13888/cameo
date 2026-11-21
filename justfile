# Default recipe — show available commands
default:
    @just --list

# Format all code
fmt:
    cargo fmt --all

# Check formatting (CI mode)
fmt-check:
    cargo fmt --all -- --check

# Run clippy lints (default features)
clippy:
    cargo clippy --all-targets -- -D warnings

# Run clippy across all CI feature combinations
clippy-all:
    #!/usr/bin/env sh
    set -e
    for features in "tmdb,cache" "anilist" "anilist,cache" "tmdb" "tmdb,anilist" "tvdb" "tmdb,anilist,tvdb,cache"; do
        echo "==> clippy --features $features"
        cargo clippy --all-targets --no-default-features --features "$features" -- -D warnings
    done

# Auto-fix clippy lints
clippy-fix:
    cargo clippy --fix --allow-dirty --allow-staged --all-targets -- -D warnings

# Check that hand-written sources under src/ stay within the file-length limit
file-length:
    ./scripts/check-file-length.sh

# Build (default features)
build:
    cargo build

# Build with specific features
build-features features:
    cargo build --no-default-features --features "{{features}}"

# Run tests (default features; needs cargo-nextest, see TESTING.md)
test:
    cargo nextest run

# Run tests with plain `cargo test` (no cargo-nextest; includes doctests)
test-cargo:
    cargo test

# Run tests with specific features
test-features features:
    cargo nextest run --no-default-features --features "{{features}}"

# Run tests across all CI feature combinations
test-all-features:
    #!/usr/bin/env sh
    set -e
    for features in "tmdb,cache" "anilist" "anilist,cache" "tmdb" "tmdb,anilist" "tvdb" "tmdb,anilist,tvdb,cache"; do
        echo "==> test --features $features"
        cargo nextest run --no-default-features --features "$features"
        cargo test --doc --no-default-features --features "$features"
    done

# Run all tests including AniList
test-all:
    cargo nextest run --features full

# Run live tests (requires TMDB_API_TOKEN in env or .env); one at a time
test-live:
    set -a; source .env; set +a && cargo nextest run --profile live --features full,live-tests

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
    cargo nextest run --features full
    cargo test --doc --features full
    cargo doc --no-deps
    cargo publish --dry-run

# Mutation-test the sources matching a glob (default: all; needs cargo-mutants)
mutants files="src/**/*.rs":
    cargo mutants --file '{{files}}'

# Mutation-test only the lines changed since a base ref (what CI runs)
mutants-diff base="origin/master":
    mkdir -p target
    git diff '{{base}}...HEAD' > target/mutants.diff
    cargo mutants --in-diff target/mutants.diff

# Mutation-test one shard of the whole tree: the same mutants as that shard of
# the weekly CI run (shard k of n, counting from 0; e.g. `just mutants-shard 3 16`).
# CI adds --in-place, which is safe only on a throwaway checkout.
mutants-shard k n:
    cargo mutants --no-shuffle --shard '{{k}}/{{n}}' --sharding round-robin

# Regenerate TMDB OpenAPI spec
fetch-openapi:
    ./scripts/fetch-openapi.sh

# Preview changelog for next release (requires release-plz CLI)
changelog:
    release-plz generate-changelog

# Install git hooks
setup:
    lefthook install
    cargo build

