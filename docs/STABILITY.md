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
- the low-level `TmdbClient` / `AniListClient` surfaces, their configs, and the
  hand-owned result types they return;
- the cache contract (`CacheBackend`, `CacheKey`, `CacheError`).

**Not** part of the stable API (may change in any release):

- anything gated behind the `live-tests` feature or documented as internal;
- exact error *messages* (the error *variants* are stable; their `Display`
  strings are not);
- the on-disk cache format and `CACHE_SCHEMA_VERSION` (the cache self-migrates);
- the vendored `generated` module — it is `pub(crate)` and unreachable.

## `#[non_exhaustive]`

Public enums and structs that model external data (models, `Genre`, error
types, configs, `Capability`, …) are `#[non_exhaustive]`. This lets us **add**
variants and fields in a minor release without it being breaking. Consequently:

- match on these enums with a `_` arm;
- construct these structs via their `new()` + `with_*` builders, not struct
  literals.

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
