## Summary

What does this change and why? Link any related issue (`Closes #123`).

## Checklist

- [ ] `cargo test --all-features` passes
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` is clean
- [ ] `cargo fmt --all` applied
- [ ] Conventional commit messages (`type: description`)
- [ ] Public items are documented with `///`
- [ ] If a `Unified*` model's serialized shape changed, bumped
      `CACHE_SCHEMA_VERSION`
- [ ] If `src/generated/` changed, it was produced by `cargo xtask codegen`
      (not hand-edited)
