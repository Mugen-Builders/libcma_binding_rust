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
- **`tests/c_parser_link.rs`** — forces libcma's `parser_impl.o` into the link so
  the libcmt dependency below cannot silently regress, and checks the C parser
  agrees with the Rust port on the canonical ether-deposit vector. The
  `riscv-link-check` CI job gained the equivalent guard: it now links a real
  riscv64 binary that calls the C parser instead of only `ar`-inspecting the
  archive.
- CI now asserts `DEPENDENCIES.lock` matches the actual submodule pins.

### Changed
- **Vendored `libcma` bumped to `v0.1.0-alpha.10` and `libcmt` to `v0.18.0`.**
  libcma's own API and implementation are unchanged between `alpha.9` and
  `alpha.10` — the release only bumps the libcmt it builds against (0.17.2 →
  0.18.0), adds `ioctl.h` to the extracted libcmt header set, and makes
  `make docker-image` pass `MACHINE_GUEST_TOOLS_VERSION` as a build-arg so the
  image's libcmt can no longer drift from the staged headers. The libcmt changes
  that reach bindgen are renames only, with no size, alignment, or field-offset
  change: `cmt_rollup_t.finish_root_hash` → `finish_outputs_merkle_root`, struct
  tag `cmt_rollup_finish` → `cmt_rollup_finish_s` (the `_t` typedef is
  unchanged), and `HTIF_YIELD_REASON_{ADVANCE,INSPECT}` →
  `..._{ADVANCE,INSPECT}_STATE`. No source change was needed in this crate.
  Note the Cartesi SDK 12 application templates also ship guest-tools 0.18.0, so
  this keeps the crate aligned with the runtime its apps execute in.
- **Vendored `libcma` bumped to the uint96 single-asset format (drive format v2;
  machine-asset-tools `v0.1.0-alpha.9`, `19a1e5e` — the merge of the `e4bfc24`
  feature branch).** The single-asset drive record widened its
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
- **`libcmt` is now linked alongside `libcma` on the real backends.** `libcma.a`
  archives only libcma's own objects; its `parser_impl.o` references libcmt's C
  ABI helpers (`cmt_abi_*`, `cmt_buf_*`), which are not in the archive, but
  `build.rs` emitted only `-lcma -lstdc++`. Because archive members are pulled
  lazily and nothing in this crate calls the C parser (`parser.rs` is pure Rust
  over `alloy`), the gap was invisible here and only broke DOWNSTREAM consumers
  that called `cma_parser_decode_advance`/`_inspect` or `cma_parser_encode_voucher`
  — with a dozen `undefined symbol: cmt_abi_*` at link time. `host-real` now
  builds and statically links `libcmt.a`; `riscv64` emits a plain `-lcmt`,
  resolved by the machine-guest-tools package in the application image (upstream
  deliberately does not build libcmt for riscv64, as its real io backend needs
  the cartesi kernel headers).
- **Stale prebuilt archives are no longer reused across submodule bumps.**
  `build.rs` cached the C++ build on the mere existence of `libcma.a`, so a bump
  of `machine-asset-tools` silently linked the previously-compiled archive
  against the new headers. The object directory is now stamped with the inputs
  that determine it (both submodule revisions, target arch, compiler overrides)
  and rebuilt from scratch when they change.
- **`+crt-static` targets now link the C++ runtime statically too.** The Cartesi
  Rust application template sets `-C target-feature=+crt-static`, which IS
  honoured for `riscv64gc-unknown-linux-gnu`: glibc links in statically. But
  `build.rs` requested libstdc++ as a *dylib*, leaving the application with a
  static libc, a lone `NEEDED libstdc++.so.6`, and therefore an interpreter of
  `/lib/ld.so.1` — a path that does not exist in the machine rootfs (Ubuntu
  riscv64 ships the loader as `/lib/ld-linux-riscv64-lp64d.so.1`). The machine
  could not exec the application at all, reporting only `dapp failed to start
  with No such file or directory`, which names neither the loader nor libstdc++.
  Under `+crt-static`, libstdc++ and libcmt are now bound statically, with each
  archive located via `<compiler> -print-file-name=` so the paths come from the
  toolchain. Verified end to end by booting a `cartesi create --template rust`
  application on a real Cartesi machine.
- **A partially-staged `third-party/libcmt` is now repaired automatically.**
  Upstream's staging target is a directory, whose mtime is always newer than the
  tarball it came from, so make treats an incomplete copy as up to date forever
  and the libcmt compile dies on a missing `src/buf.c`.

### Security / hardening
- Mutual-exclusion guards: enabling more than one backend feature now fails at
  compile time (`compile_error!`) instead of silently letting the mock win in a
  real build.
- `build.rs` verifies the nlohmann/json header it downloads against a SHA-256
  pinned in that file — it is the one build input fetched over the network
  rather than pinned as a submodule. (Earlier wording here claimed a checksum
  over "the vendored/built libcma source"; libcma is pinned by submodule commit
  SHA, not checksummed.)
- `SECURITY.md` documents why the C/C++ dependencies are pinned by commit SHA
  rather than tag name: upstream tags are mutable, and machine-asset-tools'
  `v0.1.0-alpha.8` has already been force-moved once.

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
