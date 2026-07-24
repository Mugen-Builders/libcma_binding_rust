//! host-real records byte-layout + reproducibility test.
//!
//! Builds/runs ONLY under `--no-default-features --features host-real` (the real C++ libcma
//! compiled for the host). Under the default `mock` backend the caller buffer is ignored, so
//! there are no records to inspect and the whole file compiles to nothing.
//!
//! ```sh
//! cargo test --no-default-features --features host-real --test host_real_records_layout -- --nocapture
//! ```
//!
//! Why this test exists: host↔machine reproducibility (the sequencer predicting the Cartesi
//! machine's ledger, the watchdog byte-comparing images, the emergency-withdrawal proof) all
//! depend on the on-drive account record having a FIXED, stable byte layout. This test locks
//! that layout down and asserts it is deterministic across runs. The layout is defined by
//! `third_party/machine-asset-tools/src/ledger_impl.h`:
//!
//! ```c
//! struct alignas(32) cma_ledger_single_balance {
//!     uint64_t balance;          // bytes  0..8   little-endian (interprocess.hpp asserts LE)
//!     cma_abi_address_t address; // bytes  8..28  20-byte owner wallet address
//!     uint8_t padding[4];        // bytes 28..32  zero pad to a 32-byte power-of-two record
//! };
//! static_assert(sizeof(cma_ledger_single_balance) == 32);
//! ```
#![cfg(feature = "host-real")]

use libcma_binding_rust::ledger::{Ledger, LedgerAsset};
use libcma_binding_rust::{Address, U256};

const MEM_LEN: usize = 4 * 1024 * 1024;
const MAX_ACCOUNTS: usize = 4096;
const RECORDS_PREFIX: usize = MAX_ACCOUNTS * 32; // 128 KiB proven records region

// Record byte layout (see module docs / ledger_impl.h). These offsets are the invariant.
const RECORD_SIZE: usize = 32;
const BALANCE_RANGE: std::ops::Range<usize> = 0..8; // u64 little-endian
const OWNER_RANGE: std::ops::Range<usize> = 8..28; // 20-byte owner address
const PAD_RANGE: std::ops::Range<usize> = 28..32; // 4-byte zero pad

fn token() -> Address {
    "0x88A2120B7068E78692C8fd12E751d610B6377E4d"
        .parse()
        .unwrap()
}
fn alice() -> Address {
    "0x1111111111111111111111111111111111111111"
        .parse()
        .unwrap()
}
fn bob() -> Address {
    "0x2222222222222222222222222222222222222222"
        .parse()
        .unwrap()
}

/// Initialise a single-asset (ERC-20) buffer-backed ledger over `buf` and credit each
/// `(owner, balance)`. The ledger state lives in `buf`'s 32-byte records prefix once this
/// returns. Mirrors the setup in `tests/host_real_smoke.rs`.
fn build_ledger(buf: &mut [u8], credits: &[(Address, u64)]) {
    let mut ledger = Ledger::new().expect("ledger init");
    ledger
        .init_single_from_buffer(buf, MAX_ACCOUNTS, LedgerAsset::Erc20(token()))
        .expect("init single buffer");
    let asset = ledger
        .retrieve_erc20_asset_via_address(token())
        .expect("asset");
    for &(owner, bal) in credits {
        let account = ledger.retrieve_account_via_address(owner).expect("account");
        ledger
            .deposit(asset, account, U256::from(bal))
            .expect("deposit");
    }
}

/// Return the 32-byte record in `buf`'s records prefix whose owner field (bytes 8..28)
/// equals `owner`, or `None` if no such record exists.
fn find_record(buf: &[u8], owner: Address) -> Option<&[u8]> {
    let want = owner.0 .0;
    buf[..RECORDS_PREFIX]
        .chunks_exact(RECORD_SIZE)
        .find(|rec| rec[OWNER_RANGE] == want)
}

/// Each credited account's 32-byte record is exactly `balance(u64 LE) | owner(20) | pad(4)`.
#[test]
fn record_layout_is_balance_owner_pad() {
    let credits = [(alice(), 250u64), (bob(), 70u64)];

    let mut buf = vec![0u8; MEM_LEN];
    build_ledger(&mut buf, &credits);

    for &(owner, bal) in &credits {
        let rec = find_record(&buf, owner).unwrap_or_else(|| {
            panic!("no record for {owner:?} — ledger state is not in the caller buffer")
        });
        eprintln!("record for {owner:?} = {}", hex::encode(rec));

        // balance: u64, little-endian, bytes 0..8
        let got = u64::from_le_bytes(rec[BALANCE_RANGE].try_into().unwrap());
        assert_eq!(got, bal, "balance field (u64 LE) mismatch for {owner:?}");

        // owner: 20-byte address, bytes 8..28
        assert_eq!(
            rec[OWNER_RANGE], owner.0 .0,
            "owner field (bytes 8..28) mismatch for {owner:?}"
        );

        // pad: 4 zero bytes, bytes 28..32
        assert_eq!(
            rec[PAD_RANGE], [0u8; 4],
            "trailing pad (bytes 28..32) must be zero for {owner:?}"
        );
    }
}

/// Identical inputs must produce byte-identical records: the reproducibility property the
/// watchdog byte-compare and the emergency-withdrawal proof depend on.
#[test]
fn records_are_deterministic_across_runs() {
    let credits = [(alice(), 250u64), (bob(), 70u64)];

    let mut buf_a = vec![0u8; MEM_LEN];
    let mut buf_b = vec![0u8; MEM_LEN];
    build_ledger(&mut buf_a, &credits);
    build_ledger(&mut buf_b, &credits);

    assert_eq!(
        buf_a[..RECORDS_PREFIX],
        buf_b[..RECORDS_PREFIX],
        "identical credits must yield byte-identical records (host reproducibility)"
    );
}
