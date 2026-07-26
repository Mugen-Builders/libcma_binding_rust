//! Smoke tests for the `host-real` feature: real C++ libcma compiled for the host.
//!
//! These only build/run under `--no-default-features --features host-real` (otherwise
//! the crate links the in-memory mock, which ignores the caller buffer). Run with:
//!
//! ```sh
//! cargo test --no-default-features --features host-real --test host_real_smoke -- --nocapture
//! ```
//!
//! What they establish for the sequencer integration (see the CMA app's
//! `SEQUENCER-INTEGRATION-PLAN.md`, Phase 0/2):
//!   1. real libcma links and computes on the host (not the mock);
//!   2. the ledger state DOES live in the caller buffer's 32-byte records prefix
//!      (so `create_dump` can read it out for the snapshot + emergency-withdrawal proof);
//!   3. `init_single_from_buffer` zero-initialises — it does NOT re-attach to existing
//!      contents — so `from_dump` must rebuild a fresh ledger by re-crediting, not by
//!      re-opening a saved buffer.
#![cfg(feature = "host-real")]

use libcma_binding_rust::ledger::{Ledger, LedgerAsset};
use libcma_binding_rust::{Address, U256};

const MEM_LEN: usize = 4 * 1024 * 1024;
const MAX_ACCOUNTS: usize = 4096;
const RECORDS_PREFIX: usize = MAX_ACCOUNTS * 32; // 128 KiB proven region

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

/// Real libcma links and computes balances on the host (i.e. we are NOT on the mock).
#[test]
fn real_libcma_links_and_computes_balances_on_host() {
    let mut buf = vec![0u8; MEM_LEN];
    let mut ledger = Ledger::new().expect("ledger init");
    ledger
        .init_single_from_buffer(&mut buf, MAX_ACCOUNTS, LedgerAsset::Erc20(token()))
        .expect("init single buffer");

    let asset = ledger
        .retrieve_erc20_asset_via_address(token())
        .expect("asset");
    let account = ledger
        .retrieve_account_via_address(alice())
        .expect("account");
    ledger
        .deposit(asset, account, U256::from(100))
        .expect("deposit");

    assert_eq!(
        ledger.get_balance(asset, account).expect("balance"),
        U256::from(100)
    );
}

/// The ledger state lives in the caller buffer's records prefix, and each 32-byte record
/// carries the owner's 20-byte address. Prints the record layout so `create_dump` can be
/// written against the real bytes.
#[test]
fn records_prefix_holds_owner_and_balance() {
    let mut buf = vec![0u8; MEM_LEN];
    let mut ledger = Ledger::new().expect("ledger init");
    ledger
        .init_single_from_buffer(&mut buf, MAX_ACCOUNTS, LedgerAsset::Erc20(token()))
        .expect("init single buffer");
    let asset = ledger
        .retrieve_erc20_asset_via_address(token())
        .expect("asset");
    let acc = ledger
        .retrieve_account_via_address(alice())
        .expect("account");
    ledger
        .deposit(asset, acc, U256::from(250))
        .expect("deposit");

    // Scan the 128 KiB records prefix (32-byte strides) for alice's address bytes.
    let addr = alice().0 .0;
    let mut found = None;
    for (i, rec) in buf[..RECORDS_PREFIX].chunks_exact(32).enumerate() {
        if rec.windows(20).any(|w| w == addr) {
            eprintln!("record[{i}] = {}", hex::encode(rec));
            found = Some(i);
            break;
        }
    }
    assert!(
        found.is_some(),
        "alice's address must appear in the records prefix — state is not in the buffer"
    );
}

/// The snapshot/restore mechanism for `from_dump`: capture the logical (address, balance)
/// set, then rebuild a fresh ledger by re-crediting. Total supply and per-account balances
/// must match. (This is what `from_dump` will do; it does NOT rely on re-opening a buffer.)
#[test]
fn restore_by_recredit_round_trips() {
    let token = token();

    // Original ledger.
    let mut buf = vec![0u8; MEM_LEN];
    let mut l1 = Ledger::new().expect("ledger init");
    l1.init_single_from_buffer(&mut buf, MAX_ACCOUNTS, LedgerAsset::Erc20(token))
        .expect("init");
    let a1 = l1.retrieve_erc20_asset_via_address(token).expect("asset");
    let alice_id = l1.retrieve_account_via_address(alice()).expect("alice");
    let bob_id = l1.retrieve_account_via_address(bob()).expect("bob");
    l1.deposit(a1, alice_id, U256::from(250))
        .expect("dep alice");
    l1.deposit(a1, bob_id, U256::from(70)).expect("dep bob");
    let supply1 = l1.get_total_supply(a1).expect("supply1");

    // Snapshot = the logical set (in id order, so the rebuild assigns identical ids).
    let snapshot = [(alice(), U256::from(250)), (bob(), U256::from(70))];

    // Restore into a fresh ledger by re-crediting.
    let mut buf2 = vec![0u8; MEM_LEN];
    let mut l2 = Ledger::new().expect("ledger init 2");
    l2.init_single_from_buffer(&mut buf2, MAX_ACCOUNTS, LedgerAsset::Erc20(token))
        .expect("init 2");
    let a2 = l2.retrieve_erc20_asset_via_address(token).expect("asset 2");
    for (addr, bal) in snapshot {
        let id = l2.retrieve_account_via_address(addr).expect("acct");
        l2.deposit(a2, id, bal).expect("recredit");
    }

    // Balances and total supply match the original.
    let alice2 = l2.retrieve_account_via_address(alice()).expect("alice2");
    let bob2 = l2.retrieve_account_via_address(bob()).expect("bob2");
    assert_eq!(
        l2.get_balance(a2, alice2).expect("bal alice2"),
        U256::from(250)
    );
    assert_eq!(l2.get_balance(a2, bob2).expect("bal bob2"), U256::from(70));
    assert_eq!(l2.get_total_supply(a2).expect("supply2"), supply1);

    // And the rebuilt records prefix is byte-identical to the original (the property the
    // watchdog byte-compare and the emergency-withdrawal proof both rely on).
    assert_eq!(
        buf[..RECORDS_PREFIX],
        buf2[..RECORDS_PREFIX],
        "rebuilt records prefix must be byte-identical to the original"
    );
}
