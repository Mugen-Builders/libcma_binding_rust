//! Rust bindings for Cartesi's `libcma` (Cartesi Machine Application tooling):
//! parse rollup inputs, build on-chain voucher payloads, and manage a ledger of
//! application assets (ETH, ERC-20, ERC-721, ERC-1155).
//!
//! # Backends
//!
//! Exactly one backend feature is compiled in — they are **mutually exclusive**,
//! enforced by the `compile_error!` guards below:
//!
//! - **`mock`** (default) — an in-memory **stub** ledger (`src/mocks.rs`). Needs
//!   no C++ toolchain, network, or RISC-V archive; for compile/plumbing tests
//!   only. It is **not** real libcma and must never be used in production.
//! - **`host-real`** — the real C++ libcma built for the host (x86_64). Used
//!   off-chain, e.g. a sequencer predicting the Cartesi machine's ledger.
//! - **`riscv64`** — the real C++ libcma cross-compiled to run inside the
//!   Cartesi machine.
//!
//! Selecting a real backend requires disabling default features, otherwise the
//! default `mock` stays enabled and silently wins (you link the stub):
//!
//! ```text
//! cargo build --no-default-features --features host-real   # or riscv64
//! ```
//!
//! # Reproducibility invariant
//!
//! `host-real` and `riscv64` compile libcma with SIMD-free / generic flags
//! (`-DBOOST_UNORDERED_DISABLE_SSE2`, `-DBOOST_UNORDERED_DISABLE_NEON`,
//! `-DBOOST_INTERPROCESS_FORCE_GENERIC_EMULATION`) so the on-disk 32-byte account
//! records (single-asset drive format v2: `balance` uint96 little-endian [low u64 |
//! high u32] | `owner` 20 bytes, no padding) are
//! byte-identical across x86_64 and riscv64. That invariant is what makes
//! off-chain prediction with `host-real` sound: the host reproduces, byte for
//! byte, exactly what the machine computes on-chain.
//!
//! See [`ledger::Ledger`] for the ledger wrapper and its thread-safety contract.

// ---------------------------------------------------------------------------
// Backend selection guard.
//
// Exactly one backend feature must be enabled — they are mutually exclusive:
//   `mock`      — in-memory STUB ledger (default; NOT real libcma, never use in production).
//   `host-real` — real C++ libcma compiled for the host (x86_64).
//   `riscv64`   — real C++ libcma cross-compiled for the Cartesi machine (riscv64).
//
// The build.rs link gate keys off `mock`, so enabling two backends (e.g. leaving the
// default `mock` on while adding `host-real`) would silently link the fake ledger. Turn
// that footgun into a hard compile error.
#[cfg(any(
    all(feature = "mock", feature = "host-real"),
    all(feature = "mock", feature = "riscv64"),
    all(feature = "host-real", feature = "riscv64"),
))]
compile_error!(
    "libcma_binding_rust: more than one backend feature is enabled, but they are mutually \
     exclusive — enable exactly one of `mock`, `host-real`, or `riscv64`; for a real build use \
     `default-features = false, features = [\"host-real\"]` (or `riscv64`). If you enabled a real \
     backend without `default-features = false`, the default `mock` feature is still on — that is \
     almost certainly the cause; disable default features."
);

#[cfg(not(any(feature = "mock", feature = "host-real", feature = "riscv64")))]
compile_error!(
    "libcma_binding_rust: no backend feature is enabled — enable exactly one of `mock`, \
     `host-real`, or `riscv64`; for a real build use `default-features = false, \
     features = [\"host-real\"]` (or `riscv64`)."
);
// ---------------------------------------------------------------------------

pub mod bindings {
    #![allow(non_upper_case_globals)]
    #![allow(non_camel_case_types)]
    #![allow(non_snake_case)]
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

#[cfg(feature = "mock")]
mod mocks;

pub mod error;
pub mod helpers;
pub mod ledger;
pub mod parser;
pub mod types;

pub use error::{LedgerError, ParserError};
pub use ledger::{
    Ledger, LedgerAsset, LedgerBufferConfig, LedgerFileConfig, LedgerSingleFileConfig,
};
pub use parser::{
    cma_parser_get_last_error_message, CmaParserBalance, CmaParserErc1155BatchDeposit,
    CmaParserErc1155BatchTransfer, CmaParserErc1155BatchWithdrawal, CmaParserErc1155SingleDeposit,
    CmaParserErc1155SingleTransfer, CmaParserErc1155SingleWithdrawal, CmaParserErc20Deposit,
    CmaParserErc20Transfer, CmaParserErc20Withdrawal, CmaParserErc721Deposit,
    CmaParserErc721Transfer, CmaParserErc721Withdrawal, CmaParserError, CmaParserEtherDeposit,
    CmaParserEtherTransfer, CmaParserEtherWithdrawal, CmaParserInput, CmaParserInputData,
    CmaParserInputType, CmaParserSupply, CmaParserUnidentifiedInput, CmaParserVoucherType,
    CmaVoucher,
};
pub use types::*;
