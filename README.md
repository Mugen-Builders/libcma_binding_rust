<br>
<p align="center">
    <img src="https://github.com/user-attachments/assets/080bb0be-060c-4813-85b4-6d9bf25af01f" align="center" width="20%">
</p>
<br>
<div align="center">
	<i>Cartesi Rollups LIBCMA Binding for Rust</i>
</div>
<div align="center">
	<!-- <b>Any Code. Ethereum's Security.</b> -->
</div>
<br>
<p align="center">
	<img src="https://img.shields.io/github/license/Mugen-Builders/libcma_binding_rust?style=default&logo=opensourceinitiative&logoColor=white&color=008DA5" alt="license">
	<img src="https://img.shields.io/github/last-commit/Mugen-Builders/libcma_binding_rustt?style=default&logo=git&logoColor=white&color=000000" alt="last-commit">
</p>

# libcma-binding-rust

LIBCMA is a Lightweight Rust utilities to parse Cartesi Machine Application (CMA) inputs, build on-chain voucher payloads to be executed on the base layer and handle core asset operations such as deposits, withdrawals, transfers, and balance checks. It acts as a plugin wallet for applicaitons and is particularly useful when you want scalable, predictable, and secure asset management in Cartesi applications that support assets like ETH, ERC20, ERC721, and ERC1155.

## Clone with submodules

Headers come from two git submodules under `third_party/`:

- **`third_party/machine-asset-tools`** — `libcma` (parser, types, ledger).
- **`third_party/machine-guest-tools`** — `libcmt` headers used by `libcma` (e.g. `libcmt/abi.h`); required for `bindgen` when generating bindings.

Clone with submodules in one step:

```bash
git clone --recurse-submodules https://github.com/Mugen-Builders/libcma_binding_rust
```

If you already cloned without submodules:

```bash
git submodule update --init --recursive
```

## Build requirements

- **Clang / libclang** — `bindgen` needs a working libclang installation (same as any typical `bindgen` setup).
- **Submodules** — `build.rs` expects both `third_party` trees to exist (after the step above).
- **Linking the real C library** — With default features disabled, the build links the static library from `third_party/machine-asset-tools/build/riscv64` (see **Feature flags**). For local development on the host, the default `native` feature uses Rust mocks instead of linking that binary.

## Feature flags

| Feature   | Default | Purpose                                                                                                                                                |
| --------- | ------- | ------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `native`  | yes     | Compiles `src/mocks.rs` shims for ledger/parser C symbols so you can build and run tests on the host (e.g. macOS) without the RISC-V `libcma` archive. |
| `riscv64` | no      | Reserved for cross-builds that link the vendored static `cma` library (see `build.rs`).                                                                |

Build without default features when you have the static library path and target set up:

```bash
cargo build --no-default-features
```

## Ledger wrapper

The `Ledger` type wraps the C `cma_ledger_*` API. Besides `Ledger::new()` (in-memory basic ledger), you can reinitialize storage using the same functions as the C library:

- `Ledger::init_from_file(path, LedgerFileConfig { ... })` → `cma_ledger_init_file`
- `Ledger::init_from_buffer(&mut [u8], LedgerBufferConfig { ... })` → `cma_ledger_init_buffer`

Exported helpers: `LedgerMemoryMode`, `LedgerFileConfig`, `LedgerBufferConfig`.

## Parser and vouchers

This crate is useful for Cartesi dApp developers who need to:

- Decode deposit payloads coming from Cartesi portals (ETH, ERC20, ERC721, ERC1155).
- Decode withdrawal / transfer / ledger inspection requests encoded as JSON and hex.
- Encode voucher payloads for on-chain token transfers (Ether, ERC20, ERC721 — ERC1155 encoding TODO).

### Core public functions

- `cma_decode_advance(input: JsonValue) -> Result<CmaParserInput, CmaParserError>`  
  Detects whether an input is a portal deposit (based on the caller address) or a user instruction (withdrawal/transfer). Returns a typed `CmaParserInput` (enum + associated structs) with parsed fields (addresses, amounts, token ids, exec_layer data).
- `cma_decode_inspect(input: JsonValue) -> Result<CmaParserInput, CmaParserError>`  
  Parses inspection (ledger) requests like `ledgerGetBalance` and `ledgerGetTotalSupply` encoded as JSON-in-hex and returns structured arguments.
- `cma_encode_voucher(req_type: CmaParserVoucherType, voucher_request: CmaParserVoucherData) -> Result<CmaVoucher, CmaParserError>`  
  Builds a `CmaVoucher` (destination, value, payload) for sending to the chain. Implemented for Ether, ERC20 and ERC721 vouchers.

### Important types

- `CmaParserInputType` — recognized request types (deposits, withdrawals, transfers, ledger queries).
- `CmaParserInputData` — enum wrapping parsed payload structs, e.g. `EtherDeposit`, `Erc20Transfer`, `EtherWithdrawal`, etc.
- `CmaParserError` — parsing error variants and helper conversions.
- `CmaVoucher` — voucher structure (destination, value, payload) ready for submission.

## Dependencies

- ethers-core = "1.0.0"
- hex = "0.4"
- json = "0.12"
- once_cell = "1.18"

## Tests

```bash
cargo test
```

Ledger integration tests exercise the `native` mock path by default.

## Notes and limitations

- ERC1155 voucher encoding is not implemented (placeholders return not implemented).
- The parser expects JSON input as received by the application; it inspects the caller and method to choose a decoding path.

## Contributing

- Open PRs for improvements, add missing voucher encodings, add more exhaustive parsing tests for edge cases.

## License

MIT (see `Cargo.toml`).
