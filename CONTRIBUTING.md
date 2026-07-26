# Contributing

Thanks for your interest in improving `libcma_binding_rust`.

## Prerequisites

Clone with submodules — the C headers live under `third_party/`:

```bash
git clone --recurse-submodules https://github.com/Mugen-Builders/libcma_binding_rust
# or, if already cloned:
git submodule update --init --recursive
```

(`build.rs` will auto-init the submodules if they are missing, but doing it
yourself is more predictable.)

## Building

The default build uses the pure-Rust **`mock`** backend — no network or C++
toolchain required:

```bash
cargo build
```

A **real** libcma build links the compiled C++ library instead of the mock:

```bash
# host (x86_64), off-chain use:
cargo build --no-default-features --features host-real

# Cartesi machine target (riscv64):
cargo build --no-default-features --features riscv64
```

Real builds additionally require **g++ ≥ 14**, GNU `make`, and **network access**
(`build.rs` fetches / compiles the archive from source). For `riscv64` you also
need the RISC-V GCC 14 cross toolchain (`g++-14-riscv64-linux-gnu`).

### Feature rule: exactly one backend

`mock`, `host-real`, and `riscv64` are **mutually exclusive** — exactly one must
be enabled, and a `compile_error!` guard enforces it. Because `mock` is a default
feature, selecting a real backend means also disabling defaults:

```bash
cargo build --no-default-features --features host-real
```

Enabling a real backend without `--no-default-features` leaves `mock` on, which
the guard rejects.

## Testing

```bash
cargo test
```

Tests run against the `mock` backend by default.

## Formatting and linting

Before opening a pull request:

```bash
cargo fmt --all
cargo clippy --all-targets
```

The repo pins `stable` via `rust-toolchain.toml` and ships a `rustfmt.toml`, so
formatting stays consistent across contributors.

## Pull requests

- Keep changes focused, and call out breaking changes clearly (the crate is
  pre-1.0, so breaking changes are allowed but should be documented).
- Update `CHANGELOG.md` under the `## [Unreleased]` section.
