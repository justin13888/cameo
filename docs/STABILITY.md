# Stability policy

This document defines what cameo's public API promises and how it evolves.

## Our promise: cameo stays on `0.x`

cameo keeps growing new providers and clients, so it intentionally keeps a `0.x`
version rather than freezing a 1.0 surface. **There is no planned 1.0.** The
`0.` prefix is the signal, and this is the promise behind it:

- **We try our best to keep every change non-breaking.** Fixes, additive
  features, and new providers ship as patch releases (`0.y.z → 0.y.(z+1)`),
  which a `cameo = "0.y"` requirement picks up automatically.
- **Big improvements or feature additions may require a breaking bump**
  (`0.y → 0.(y+1)`). We take one only when it buys a real improvement; the
  change is marked `!` in its commit and pull-request title and listed in
  `CHANGELOG.md`, so upgrading is a deliberate, documented step.

This follows Cargo's SemVer rules for `0.x` crates, where the minor number is the
compatibility boundary. CI enforces it: `cargo semver-checks` rejects a breaking
API change unless a breaking marker has been declared since the last release.

## Stable surface

Everything reachable from the crate root (`cameo::…`) that is not explicitly
marked otherwise is part of the stable API and follows the promise above and
[SemVer](https://semver.org/):

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
  `AniListGqlError`'s `path` / `extensions`), `jiff` (civil dates:
  `PartialDate`'s conversions and the TMDB discover date filters),
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

## `#[non_exhaustive]`

Public enums and structs that model external data (models, `Genre`, error
types, configs, `Capability`, …) are `#[non_exhaustive]`. This lets us **add**
variants and fields in a patch release without it being breaking. Consequently:

- match on these enums with a `_` arm;
- construct input models and configs via their `new()` + `with_*` builders,
  not struct literals; a config with no required fields (e.g.
  `CacheTtlConfig`) starts from `Default::default()` instead of `new()`.
  Output-only types (provider responses and error details) are not
  constructed by callers at all.

## What counts as a breaking change

Requires a breaking (`0.y → 0.(y+1)`) release:

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

Where practical, items are removed only after a deprecation period: an item is
marked `#[deprecated(since = "0.y.z", note = "use … instead")]` for **at least
one release** before removal in the next breaking release. Deprecations are called out in
`CHANGELOG.md`.

## MSRV

The minimum supported Rust version is stated in the README and `Cargo.toml`
(`rust-version`). An MSRV increase ships only in a breaking (`0.(y+1)`)
release, never in a patch, and is noted in the changelog.
