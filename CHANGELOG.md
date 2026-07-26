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
- **Vendored `libcma` bumped to the uint96 single-asset format (drive format v2;
  machine-asset-tools `e4bfc24`).** The single-asset drive record widened its
  balance from `uint64` to `uint96`, consuming the former 4-byte pad: the 32-byte
  record is now `balance_lo (u64 LE) | balance_hi (u32 LE) | owner (20B)`, with the
  owner moved from offset 8 to **offset 12**. Total supply and virtual (internal
  account-id) balances widened to full 256-bit. The public C API (and therefore the
  Rust wrapper surface) is UNCHANGED — deposits/withdrawals/balances already used
  256-bit `cma_amount_t` at the boundary; only code that parses the raw 32-byte
  records must adopt the new offsets. **The on-drive format is not backward
  compatible** (`MemoryFooter::VERSION` 1 → 2): a v1 drive would be silently
  misread. Downstream that reads the records image directly (e.g. a sequencer's
  `create_dump` / snapshot parser and the emergency-withdrawal output builder) MUST
  be updated to the offset-12 owner and uint96 balance.
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
