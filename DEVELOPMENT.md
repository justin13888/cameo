# Development with Cameo

Prerequisite: rustup, lefthook

## Setup

```bash
lefthook install
cargo build
```

## Common Commands

```bash
# Build
cargo build
cargo build --no-default-features --features anilist  # AniList only

# Test
cargo test                           # unit + wiremock tests
cargo test --features anilist        # include AniList tests

# Live tests (requires API tokens)
set -a; source .env; set +a && cargo test --features 'anilist,live-tests' -- --test-threads=1

# Docs
cargo doc --no-deps --open

# Run examples
TMDB_API_TOKEN=xxx cargo run --example facade_showcase -- 'Inception'
TMDB_API_TOKEN=xxx cargo run --example tmdb_lowlevel -- 'Inception'
cargo run --example anilist_showcase --features anilist -- 'Your Name'
```

## Notes

- Lefthook is configured with pre-commit and pre-push hooks
- AniList rate limit: 90 req/min — run live tests with `--test-threads=1`
- See TESTING.md for detailed test documentation
