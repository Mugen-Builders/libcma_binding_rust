# Repository structure

This document describes how the **`libcma_binding_rust`** crate is laid out: Rust sources, vendored C headers, and how the build ties them together.

## Top-level layout

```
libcma_binding_rust/          # crate root (see Cargo.toml [package] name)
├── Cargo.toml
├── Cargo.lock
├── build.rs                  # bindgen + optional link of static libcma
├── wrapper.h                 # includes libcmt + libcma headers for bindings
├── README.md
├── STRUCTURE.md
├── .gitmodules               # submodule URLs for third_party/*
├── src/
│   ├── lib.rs                # crate root, re-exports public API (includes generated bindings from OUT_DIR)
│   ├── error.rs              # LedgerError, ParserError
│   ├── types.rs              # Address/U256 helpers, ledger enums, etc.
│   ├── ledger.rs             # Ledger wrapper + file/buffer init configs
│   ├── parser.rs             # High-level parser / voucher helpers
│   ├── helpers.rs            # Shared helpers
│   └── mocks.rs              # #[cfg(feature = "mock")] C ABI stub shims for tests
├── tests/
│   ├── ledger_tests.rs       # Ledger behavior (mock-backed by default)
│   └── parser_tests.rs       # Parser / encoding tests
└── third_party/              # git submodules (must be initialized)
    ├── machine-asset-tools/  # Mugen-Builders machine-asset-tools → libcma headers (and optional RISC-V lib)
    └── machine-guest-tools/  # cartesi/machine-guest-tools → libcmt headers under sys-utils/libcmt/include
```

There is **no** checked-in `lib/cpp-build` tree in this layout: bindgen runs against headers under `third_party/`, and linking the real archive is optional (see below).

## Submodule roles

| Path | Upstream (typical) | Role |
|------|-------------------|------|
| `third_party/machine-asset-tools` | [machine-asset-tools](https://github.com/Mugen-Builders/machine-asset-tools) | `include/libcma/*.h` and, when built, `build/riscv64/libcma.a` (or project-specific layout) for `--no-default-features` links. |
| `third_party/machine-guest-tools` | [machine-guest-tools](https://github.com/cartesi/machine-guest-tools) | `sys-utils/libcmt/include` so includes like `libcmt/abi.h` resolve during bindgen. |

`wrapper.h` pulls in libcmt first, then libcma (`parser.h`, `types.h`, `ledger.h`).

## Build pipeline (`build.rs`)

1. **Include paths** passed to clang/bindgen:
   - `third_party/machine-asset-tools/include`
   - `third_party/machine-guest-tools/sys-utils/libcmt/include`
2. **Header root** — `wrapper.h` at the crate root.
3. **Generated output** — `$OUT_DIR/bindings.rs` (included from `src/lib.rs` inside the `bindings` module).
4. **Linking** — The link gate keys off the **`mock`** feature. With **`mock` enabled (default)**, `build.rs` links no C++ archive; `src/mocks.rs` supplies compatible `#[no_mangle]` stub symbols for development and `cargo test` on the host. With a **real backend** — `host-real` (host x86_64) or `riscv64` (Cartesi machine), each requiring `--no-default-features` — `build.rs` builds and links the static `cma` archive instead of the mock (for `riscv64` it adds `-L third_party/machine-asset-tools/build/riscv64` and links `static=cma`).

## Feature flags

Three **mutually exclusive** backends; exactly one must be enabled (a `compile_error!` in `src/lib.rs` enforces this). Selecting a real backend requires `default-features = false` — otherwise the default `mock` stays on and silently wins.

- **`mock` (default)** — Compiles `src/mocks.rs`: an in-memory **stub** ledger for host builds and unit/integration tests without any C++ toolchain or the RISC-V static library. Not real libcma; never for production.
- **`host-real`** — Builds and links the **real** C++ `libcma` for the host (x86_64). Used off-chain, e.g. a sequencer predicting the machine's ledger. Built SIMD-free so its 32-byte account records are byte-identical to the `riscv64` build.
- **`riscv64`** — Cross-compiles and links the **real** C++ `libcma` for the Cartesi machine (riscv64).

## Public surface (`src/lib.rs`)

Re-exports include:

- `Ledger`, `LedgerError`, `LedgerFileConfig`, `LedgerBufferConfig`, `LedgerMemoryMode`
- Parser types and helpers (`CmaParserInput`, `CmaVoucher`, …) and `ParserError`
- `types::*` (addresses, amounts, ledger enums, etc.)

## Tests

- **`tests/ledger_tests.rs`** — Ledger API; uses mock implementations when the `mock` backend (default) is on.
- **`tests/parser_tests.rs`** — Parser and voucher-related coverage.

Run: `cargo test`.

## Historical note

Older revisions described a `lib/` vendor tree and the name `cma-rust-parser`. The current crate is **`libcma_binding_rust`** and vendors headers via **`third_party/`** submodules as above.
