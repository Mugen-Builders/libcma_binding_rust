<br>
<p align="center">
    <img src="https://github.com/user-attachments/assets/080bb0be-060c-4813-85b4-6d9bf25af01f" align="center" width="20%">
</p>
<br>
<div align="center">
	<i>Cartesi Rollups LIBCMA Binding for Rust</i>
</div>
<br>

# libcma-binding-rust

LIBCMA is a lightweight Rust binding for Cartesi Machine Application (CMA) tooling: parse rollup inputs, build on-chain voucher payloads, and manage application assets (deposits, withdrawals, transfers, balances) for ETH, ERC-20, ERC-721, and ERC-1155.

## Clone with submodules

Headers come from git submodules under `third_party/`:

- **`third_party/machine-asset-tools`** — `libcma` (parser, types, ledger)
- **`third_party/machine-guest-tools`** — `libcmt` headers used by `libcma`

```bash
git clone --recurse-submodules https://github.com/Mugen-Builders/libcma_binding_rust
# or, if already cloned:
git submodule update --init --recursive
```

## Build requirements

A plain `cargo build` works out of the box — `build.rs` takes care of the fiddly parts:

- **Submodules auto-init.** If `third_party/` is empty (you cloned without `--recurse-submodules`), `build.rs` runs `git submodule update --init --recursive` for you.
- **bindgen clang headers auto-fallback.** `bindgen` needs the compiler's builtin headers (`stdbool.h`, …). If your `libclang` ships without them (e.g. you have `libclang1` but not `libclang-common-*-dev`), `build.rs` falls back to GCC's builtin header dir automatically — no `BINDGEN_EXTRA_CLANG_ARGS` needed. Set that env var yourself to override.

So the only hard host requirement for the default build is:

- **A Rust toolchain + `libclang`** (for `bindgen`) and **`git`** (for the submodule fetch).

For the **`riscv64`** build (linking the real C++ `libcma`), `build.rs` also cross-compiles the static archive from source if it isn't already present. That path additionally needs:

- **GNU `make`, `wget`, and network access**
- **The RISC-V GCC 14 cross toolchain** — `g++-14-riscv64-linux-gnu` / `gcc-14-riscv64-linux-gnu` (libcma's C++ source requires GCC ≥ 14). Override the compiler names with `CMA_RISCV64_CXX` / `CMA_RISCV64_CC`.

The Cartesi SDK / app Docker image used to build the machine already provides all of these.

## Feature flags

The crate has **three mutually exclusive backends**. Exactly one must be enabled;
enabling zero or more than one is a hard `compile_error!` (enforced at the top of
`src/lib.rs`).

| Feature     | Default | Real libcma?             | When to use |
| ----------- | ------- | ------------------------ | ----------- |
| `mock`      | yes     | No — in-memory **stub**  | Host development and `cargo test`: compiles the thread-local stubs in `src/mocks.rs` so the crate builds and the plumbing/parser tests run with no C++ toolchain, no network, and no RISC-V archive. **Never use in production — it is not a real ledger.** |
| `host-real` | no      | Yes (host, x86_64)       | Running the **real** C++ `libcma` off-chain on the host, e.g. a sequencer predicting the Cartesi machine's ledger state. `build.rs` builds and links the real static archive for the host. |
| `riscv64`   | no      | Yes (Cartesi machine)    | Running the **real** C++ `libcma` **inside** the Cartesi machine. `build.rs` cross-compiles `build/riscv64/libcma.a` from the submodule source if it isn't already present (needs the RISC-V GCC 14 cross toolchain). |

### Selecting a real backend (important footgun)

The backends are mutually exclusive **and** `mock` is a default feature, so the
link gate in `build.rs` keys off `mock`. To build against the real `libcma` you
MUST also turn default features off — otherwise the default `mock` stays enabled
and you silently link the stub instead of the real ledger:

```bash
# real libcma on the host (off-chain, e.g. sequencer prediction)
cargo build --no-default-features --features host-real

# real libcma cross-compiled for the Cartesi machine
cargo build --no-default-features --features riscv64
```

In `Cargo.toml`:

```toml
libcma_binding_rust = { version = "...", default-features = false, features = ["host-real"] }  # or "riscv64"
```

If you forget `default-features = false`, enabling `host-real` or `riscv64`
alongside the default `mock` trips the mutual-exclusivity `compile_error!` — read
its message; the fix is to disable default features.

### Determinism / reproducibility

`host-real` and `riscv64` compile the C++ `libcma` with SIMD-free / generic flags
(`-DBOOST_UNORDERED_DISABLE_SSE2`, `-DBOOST_UNORDERED_DISABLE_NEON`,
`-DBOOST_INTERPROCESS_FORCE_GENERIC_EMULATION`). This makes the on-disk 32-byte
account records (single-asset drive format v2: `balance` uint96 little-endian
[low u64 | high u32] | `owner` 20 bytes, no padding) **byte-identical** across
x86_64 and riscv64. That invariant is what
makes off-chain prediction with `host-real` sound: the host reproduces, byte for
byte, exactly what the machine computes on-chain.

### Thread safety

`Ledger` wraps a self-referential C++ object (Boost.Interprocess) held on the
heap for relocation safety, and is therefore **`!Send` / `!Sync`**. Do not move
or share a `Ledger` across threads without external synchronization. Downstream
code that needs `Send` typically wraps the `Ledger` in a mutex together with its
own `unsafe impl Send`.

## Ledger wrapper

`Ledger` wraps `cma_ledger_*` with helpers for file/buffer initialization, asset/account retrieval, deposit/withdraw/transfer, balance, and total supply.

- `retrieve_ether_assets()` uses `AssetType::Base`
- `AssetType` also supports `TokenAddress`, `TokenAddressId`, and `TokenAddressIdAmount`
- `RetrieveOperation::FindAndRemove` is supported

## Parser and vouchers

Pure-Rust parser aligned with `machine-asset-tools` / Cartesi Rollups v2.0:

- Portal deposit decoding (packed + ABI tails for ERC-721/1155)
- Auto-decode withdrawals/transfers by function selector
- Inspect decoding for `ledger_getBalance` and `ledger_getTotalSupply`
- Voucher encoding for Ether, ERC-20, ERC-721 (`safeTransferFrom`), ERC-1155 single/batch (`safeTransferFrom` / `safeBatchTransferFrom`)

### Core public functions

- `cma_decode_advance(req_type, input) -> Result<CmaParserInput, CmaParserError>`
- `cma_decode_inspect(input) -> Result<CmaParserInput, CmaParserError>`
- `cma_encode_voucher(req_type, app_address, voucher_request) -> Result<CmaVoucher, CmaParserError>`

`CmaVoucher` fields: `destination`, `value` (wei for ether vouchers), `payload`.

Inspect params are flat JSON strings, e.g.:

```json
{"method":"ledger_getBalance","params":["0x...account...","0x...token...","0x1"]}
```

## Tests

```bash
cargo test
```

- `tests/parser_tests.rs` — integration tests against the pure-Rust parser
- `tests/parser_vectors.rs` — vectors ported from `third_party/machine-asset-tools/tests/parser.c`
- `tests/ledger_tests.rs` — ledger tests via the `mock` backend

CI runs the `mock`-backend tests on every push/PR and attempts an riscv64 link check when `libcma` can be built.

## License

MIT (see `Cargo.toml`).
