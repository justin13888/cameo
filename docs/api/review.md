# v1 public API review

A pre-1.0 pass over the entire public surface against the
[Rust API Guidelines](https://rust-lang.github.io/api-guidelines/) and
[docs/STABILITY.md](../STABILITY.md). The full surface is snapshotted in
[`public-api.txt`](public-api.txt) (regenerate with
`cargo public-api --features full`); `cargo semver-checks` gates it in CI.

Findings are **fixed** (addressed for v1) or **waived** (intentional, with
rationale). Nothing is left open.

## Fixed

- **No leaked generated types (C-STABLE).** The `progenitor`-generated TMDB
  client is fully vendored behind hand-owned/unified types; the snapshot
  contains zero `generated::` paths. Provider methods return `Unified*` or
  hand-owned models (`TmdbCredits`, `TmdbImages`, …), never generated structs.
- **`#[must_use]` on builders and configs.** `TmdbConfig`, `DiscoverMoviesBuilder`,
  and `DiscoverTvBuilder` are `#[must_use]` — a built-but-dropped config or an
  un-`.execute()`d query is a bug and now warns.
- **Future-proof enums/structs (C-STRUCT-PRIVATE, C-FUTURE-PROOF).** Every
  public model, error, and config enum is `#[non_exhaustive]`; construction goes
  through `new()` + `with_*`, so adding fields/variants is non-breaking.
- **Honest optionality.** Fields absent from a provider are `Option` (dates →
  `Option<PartialDate>`, `vote_count`/`adult`/season+episode counts) rather than
  sentinel defaults, so "unknown" and "zero" are distinct.
- **Doc contract accuracy (C-EXAMPLE, C-LINK).** Removed stale claims (model date
  fields are no longer described as `YYYY-MM-DD` strings — they are
  `PartialDate`); `type_` documents the classification only now that
  `media_format` exists; intra-doc links resolve.
- **Errors are inspectable (C-GOOD-ERR).** `ProviderError`, `TmdbError`,
  `AniListError`, `CacheError`, and `CameoClientError` all implement
  `std::error::Error` (via `thiserror`), are `#[non_exhaustive]`, and preserve
  sources.
- **`async_trait` re-exported.** External providers implement the capability
  traits without a direct `async-trait` dependency or a version-match hazard.

## Waived

- **Models are not `#[must_use]`.** `Unified*` values are plain data that callers
  legitimately inspect field-by-field and discard; type-level `#[must_use]` would
  be noise. The consuming types (configs, builders) carry it instead.
- **Capability traits are not sealed.** Sealing would defeat the entire design —
  third-party providers *must* be able to implement them (see
  [providers.md](../providers.md)). Evolution is handled by `#[non_exhaustive]`
  models and additive default methods, not sealing.
- **`i32`/`i64` id widths in some `Unknown*`/TMDB-native spots.** These mirror the
  upstream TMDB wire types deliberately; normalizing to `u64` everywhere would
  lose fidelity for the raw-id escape hatches. Unified ids use `NativeId`.
- **`PartialDate` is not `chrono::NaiveDate`.** Providers routinely return
  year-only or year-month dates; a full `NaiveDate` cannot represent them. Lossy
  conversion is offered explicitly instead of imposed.
