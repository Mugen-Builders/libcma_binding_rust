# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

While the crate is pre-1.0 (`0.0.x`), any release may contain breaking changes.

## [Unreleased]

### Added
- **Single-asset ledger API.** New bindings and `Ledger` wrapper support for the
  single-asset `cma` ledger.
- **`host-real` feature.** Builds and links the real C++ `libcma` for the host
  (x86_64) instead of the mock, for off-chain use (e.g. a sequencer predicting
  the machine's ledger). Complements the existing `riscv64` cross-build path.
- Packaging metadata for crates.io / docs.rs: `LICENSE` (MIT), `rust-version`
  (MSRV `1.74`), a `documentation` link, and `[package.metadata.docs.rs]` (an
  offline `mock`-only docs build). Added `CHANGELOG.md`, `CONTRIBUTING.md`,
  `SECURITY.md`, `deny.toml`, `rust-toolchain.toml`, and `rustfmt.toml`.

### Changed
- **BREAKING: reshaped the ledger API around a single-asset ledger.** Removed
  `LedgerMemoryMode` and reshaped `LedgerFileConfig`. Code that constructed a
  ledger via the old memory-mode / file-config shape must be updated.
- **BREAKING: renamed the default mock feature `native` → `mock`.** The default
  backend is now `mock`. Update any `--features native` usage accordingly. The
  three mutually-exclusive backends are now `mock` (default), `host-real`, and
  `riscv64`.

### Fixed
- **Relocation safety for `cma_ledger_t`.** The self-referential C++ ledger is
  now boxed so it is not moved after construction, preventing dangling internal
  self-pointers.

### Security / hardening
- Mutual-exclusion guards: enabling more than one backend feature now fails at
  compile time (`compile_error!`) instead of silently letting the mock win in a
  real build.
- `build.rs` verifies a checksum of the vendored/built libcma source.

### Changed (breaking)
- **Migrated off the EOL `ethers-rs` onto `alloy`** (`alloy-primitives` +
  `alloy-dyn-abi`). The public `Address` / `U256` types are now
  `alloy_primitives::{Address, U256}` — a breaking change for downstream code
  that used the ethers-typed API (hence the `0.0.1` → `0.1.0` bump). The ABI
  parser (`parser.rs`) was reimplemented over `alloy-dyn-abi`; byte-for-byte
  equivalence is pinned by the existing parser test vectors (all pass).

### Known issues / tech debt
- **`json` (0.12)** — largely unmaintained (RUSTSEC-2022-0081). Planned
  migration to `serde_json`. Surfaced by `deny.toml`.

[Unreleased]: https://github.com/Mugen-Builders/libcma_binding_rust/commits/main
