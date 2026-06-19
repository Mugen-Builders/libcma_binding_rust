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

- **Clang / libclang** for `bindgen`
- **Submodules** present before building
- **Real C library** — disable default features to link the static `libcma` archive from `third_party/machine-asset-tools/build/riscv64`

## Feature flags

| Feature   | Default | Purpose |
| --------- | ------- | ------- |
| `native`  | yes     | Compiles `src/mocks.rs` shims so host tests run without the RISC-V `libcma` archive |
| `riscv64` | no      | Cross-build path that links the vendored static `cma` library |

```bash
cargo build --no-default-features --features riscv64
```

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
- `tests/ledger_tests.rs` — ledger tests via native mocks

CI runs native tests on every push/PR and attempts an riscv64 link check when `libcma` can be built.

## License

MIT (see `Cargo.toml`).
