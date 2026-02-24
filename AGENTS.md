# cameo

API client SDK for an internal/private API.

## Tech Stack

- **Runtime:** Rust
- **Language:** Rust
- **Package Manager:** Cargo
- **Formatter:** rustfmt
- **Linter:** Clippy

## Project Structure

```
cameo/
├── src/
│   └── lib.rs          # Library entry point
├── .github/
│   └── workflows/
│       └── ci.yml      # GitHub Actions CI
├── .gitignore
├── Cargo.toml
├── LICENSE-MIT
├── LICENSE-APACHE
├── rustfmt.toml
└── CLAUDE.md
```

## Development

### Setup

```bash
cargo build
```

### Test

```bash
cargo test
```

### Format

```bash
cargo fmt
```

### Lint

```bash
cargo clippy --all-targets --all-features
```

### Lint Fix

```bash
cargo clippy --all-targets --all-features --fix
```

## Conventions

- Use strict Rust — avoid `unwrap()` in library code; prefer `?` and proper error types
- Write tests for all public API surface
- Use conventional commits (`type: description`)
- Keep functions small and focused
- Document all public items with `///` doc comments
- Errors should implement `std::error::Error` and be exposed in the crate's public API

## Architecture

This is a Rust library crate exposing a typed, ergonomic client for an internal API.
Structure suggestions:
- `src/lib.rs` — public API re-exports
- `src/client.rs` — main client struct and configuration
- `src/error.rs` — unified error type
- `src/models/` — request/response types
- `src/endpoints/` — per-resource endpoint implementations
