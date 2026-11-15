# Stability policy

This document defines what cameo's public API promises once **1.0** ships, and
how it evolves. Before 1.0, minor `0.x` releases may contain breaking changes,
but we already develop against these rules.

## Stable surface

Everything reachable from the crate root (`cameo::…`) that is not explicitly
marked otherwise is part of the stable API and follows [SemVer](https://semver.org/):

- the unified models (`Unified*`, `Genre`, `Gender`, `PartialDate`, `MediaId`,
  `Page`, …) and their public methods/builders;
- the provider capability traits (`SearchProvider`, `DetailProvider`, …) and the
  `CameoClient` facade;
- the low-level `TmdbClient` / `AniListClient` / `TvdbClient` surfaces, their configs, and the
  hand-owned result types they return;
- the cache contract (`CacheBackend`, `CacheKey`, `CacheError`).

### Canonical paths

Each public item is reachable through its area module, and the most common
items also through the crate root. These are the only public paths:

| Module | Holds |
| --- | --- |
| `cameo` (crate root) | the common items of every module below |
| `cameo::unified` | models, `Genre`, `PartialDate`, `MediaId`, the provider traits, `CameoClient`, and `TimeWindow` |
| `cameo::core` | `Page`, `PageToken`, `into_stream`, `ProviderError`, `TransportError`, `DecodeError`, `TimeWindow` |
| `cameo::providers` | `RateLimit`, `RetryPolicy` |
| `cameo::providers::tmdb` | `TmdbClient`, its config, errors, discover builders, image URL helpers, and result types |
| `cameo::providers::anilist` | `AniListClient`, its config, errors, and the anime discovery builder |
| `cameo::providers::tvdb` | `TvdbClient`, its config, errors, and records |
| `cameo::cache` | `CacheBackend`, `CacheKey`, `CacheError`, `CacheTtlConfig`, `SqliteCache` |

The modules that hold the definitions (`unified::models`, `core::pagination`,
`providers::tmdb::client`, `cache::key`, …) are private, so moving an item
between files is never a public change.

**Not** part of the stable API (may change in any release):

- anything gated behind the `live-tests` feature or documented as internal;
- exact error *messages* (the error *variants* are stable; their `Display`
  strings are not);
- the on-disk cache format (self-migrating);
- the vendored `generated` module — it is `pub(crate)` and unreachable.

## Public dependencies

A type from another crate that appears in cameo's public API makes that
crate's breaking releases cameo's breaking releases. cameo therefore keeps
third-party types out of its API unless they are the deliberate vocabulary of
the API:

- **Deliberately public:** `serde` (the models derive `Serialize` /
  `Deserialize`), `serde_json::Value` (the `CacheBackend` value contract and
  `AniListGqlError`'s `path` / `extensions`), `chrono` (dates and timestamps),
  `futures`' `Stream` trait (returned by `into_stream`), and `async_trait`
  (re-exported for trait implementers). A major release of
  one of these is a breaking cameo release.
- **Kept private:** the HTTP client (`reqwest`) and the JSON parser's error
  type (`serde_json::Error`). Errors from them are wrapped in cameo's own
  `TransportError` and `DecodeError`, which classify and describe the failure
  and return the original from `std::error::Error::source()` where one can be
  kept. Downcasting that source to the dependency's type is possible, but it
  is not part of the stable API: the source's type may change when the
  dependency does, without a breaking cameo release.

`TvdbError::Http` and `TvdbError::Deserialization` still carry `reqwest::Error`
and `serde_json::Error`; they move to the same wrappers in a later release.

## `#[non_exhaustive]`

Public enums and structs that model external data (models, `Genre`, error
types, configs, `Capability`, …) are `#[non_exhaustive]`. This lets us **add**
variants and fields in a minor release without it being breaking. Consequently:

- match on these enums with a `_` arm;
- construct input models and configs via their `new()` + `with_*` builders,
  not struct literals; output-only types (provider responses and error
  details) are not constructed by callers at all.

## What counts as a breaking change

Requires a major version bump (post-1.0):

- removing or renaming a public item, or changing a function signature;
- adding a required method to a public trait, or otherwise making an existing
  downstream impl fail to compile;
- changing the serialized shape of a type in a way that breaks round-tripping.

Explicitly **not** breaking:

- adding a variant/field to a `#[non_exhaustive]` type;
- adding a new inherent method, trait, or provider;
- changing an error's `Display` text or internal behavior that preserves the
  documented contract.

## Deprecation process

Post-1.0, items are removed only after a deprecation period: an item is marked
`#[deprecated(since = "x.y", note = "use … instead")]` for **at least one minor
release** before removal in the next major. Deprecations are called out in
`CHANGELOG.md`.

## MSRV

The minimum supported Rust version is stated in the README and `Cargo.toml`
(`rust-version`). An MSRV increase is treated as a minor-version change, not a
patch, and is noted in the changelog.
