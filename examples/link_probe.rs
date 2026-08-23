//! A minimal binary that touches every native symbol group this crate links against.
//!
//! Its purpose is to be *linked*, not to be useful. `cargo build` on a library only ever
//! produces an rlib, and rlib members — like static archive members — are pulled lazily, so a
//! library build cannot detect a missing or wrongly-bound native dependency. Only linking an
//! executable does.
//!
//! CI cross-builds this for `riscv64gc-unknown-linux-gnu` with `+crt-static` and then asserts
//! the result has no `PT_INTERP` and no `NEEDED` entries. That combination is what the Cartesi
//! machine requires: its rootfs has no `/lib/ld.so.1`, so an application with any dynamic
//! dependency left cannot be exec'd at all — and the machine reports that as nothing more
//! informative than `dapp failed to start with No such file or directory`.
//!
//! Run it anywhere to check the same things dynamically:
//!
//! ```sh
//! cargo run --example link_probe --no-default-features --features host-real
//! ```

use libcma_binding_rust::ledger::{Ledger, LedgerAsset};
use libcma_binding_rust::{bindings, Address, U256};

fn main() {
    // 1. The C++ ledger: pulls ledger.o / ledger_impl.o and, with them, libstdc++.
    let token: Address = "0x88A2120B7068E78692C8fd12E751d610B6377E4d"
        .parse()
        .unwrap();
    let holder: Address = "0x1111111111111111111111111111111111111111"
        .parse()
        .unwrap();

    let mut buf = vec![0u8; 4 * 1024 * 1024];
    let mut ledger = Ledger::new().expect("ledger init");
    ledger
        .init_single_from_buffer(&mut buf, 4096, LedgerAsset::Erc20(token))
        .expect("init single-asset ledger");
    let asset = ledger
        .retrieve_erc20_asset_via_address(token)
        .expect("asset");
    let account = ledger
        .retrieve_account_via_address(holder)
        .expect("account");
    ledger
        .deposit(asset, account, U256::from(7))
        .expect("deposit");
    let balance = ledger.get_balance(asset, account).expect("balance");
    assert_eq!(balance, U256::from(7));

    // 2. The C parser: pulls parser_impl.o and, with it, libcmt's cmt_abi_* / cmt_buf_*.
    let mut payload = [0u8; 52];
    payload[19] = 1; // sender
    payload[51] = 4; // value
    let mut advance = unsafe { std::mem::zeroed::<bindings::cmt_rollup_advance_t>() };
    advance.payload = bindings::cmt_abi_bytes_t {
        length: payload.len(),
        data: payload.as_mut_ptr().cast(),
    };
    let mut decoded = unsafe { std::mem::zeroed::<bindings::cma_parser_input_t>() };
    let rc = unsafe {
        bindings::cma_parser_decode_advance(
            bindings::cma_parser_input_type_t_CMA_PARSER_INPUT_TYPE_ETHER_DEPOSIT,
            &advance,
            &mut decoded,
        )
    };
    assert_eq!(
        rc, 0,
        "C parser rejected the canonical ether-deposit vector"
    );

    println!("link probe OK: balance={balance}, C parser rc={rc}");
}
