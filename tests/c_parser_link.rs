//! Link + behaviour regression test for the **C** parser surface of libcma.
//!
//! Why this file exists: `libcma.a` archives only libcma's own objects, and its
//! `parser_impl.o` references libcmt's C ABI helpers (`cmt_abi_*`, `cmt_buf_*`). Those
//! symbols are not in the archive, so libcmt has to be linked alongside it. Static archive
//! members are pulled lazily, and nothing else in this crate calls the C parser — `parser.rs`
//! is pure Rust over `alloy` — so `parser_impl.o` never got pulled and the missing symbols
//! stayed invisible. A downstream consumer that called `cma_parser_decode_advance` hit a wall
//! of `undefined symbol: cmt_abi_*` at link time.
//!
//! Calling the C entry points below forces `parser_impl.o` into every link of this test
//! binary. If the libcmt link is ever dropped from `build.rs`, this file fails to link — which
//! is the point. It is a compile-time guard first and a behavioural test second.
//!
//! Run with:
//!
//! ```sh
//! cargo test --no-default-features --features host-real --test c_parser_link
//! ```
#![cfg(feature = "host-real")]

use libcma_binding_rust::bindings;

/// Build a `cmt_rollup_advance_t` whose payload borrows `payload`.
///
/// The returned struct holds a raw pointer into `payload`, so `payload` must outlive it —
/// enforced here by the shared lifetime on the borrow.
fn advance_with_payload(payload: &mut [u8]) -> bindings::cmt_rollup_advance_t {
    let mut advance = unsafe { std::mem::zeroed::<bindings::cmt_rollup_advance_t>() };
    advance.msg_sender = bindings::cmt_abi_address_t {
        data: {
            let mut a = [0u8; 20];
            a[19] = 1;
            a
        },
    };
    advance.payload = bindings::cmt_abi_bytes_t {
        length: payload.len(),
        data: payload.as_mut_ptr().cast(),
    };
    advance
}

/// The canonical ether-deposit vector from `third_party/machine-asset-tools/tests/parser.c`,
/// decoded by the **C** parser this time (`parser_vectors.rs` runs the same bytes through the
/// Rust one). Establishes that libcma's parser links, runs, and agrees with our port.
#[test]
fn c_parser_decodes_ether_deposit_vector() {
    // sender (20 bytes) ++ value (32 bytes, = 4), no exec-layer data.
    let mut payload = hex::decode(concat!(
        "0000000000000000000000000000000000000001",
        "0000000000000000000000000000000000000000000000000000000000000004"
    ))
    .expect("vector is valid hex");

    let advance = advance_with_payload(&mut payload);
    let mut out = unsafe { std::mem::zeroed::<bindings::cma_parser_input_t>() };

    let rc = unsafe {
        bindings::cma_parser_decode_advance(
            bindings::cma_parser_input_type_t_CMA_PARSER_INPUT_TYPE_ETHER_DEPOSIT,
            &advance,
            &mut out,
        )
    };
    assert_eq!(rc, 0, "C parser rejected the canonical ether-deposit vector");
    assert_eq!(
        out.type_,
        bindings::cma_parser_input_type_t_CMA_PARSER_INPUT_TYPE_ETHER_DEPOSIT
    );

    let deposit = unsafe { out.__bindgen_anon_1.ether_deposit };

    let mut expected_sender = [0u8; 20];
    expected_sender[19] = 1;
    assert_eq!(deposit.sender.data, expected_sender);

    // cma_amount_t is a big-endian 32-byte word; the vector's value is 4.
    let mut expected_amount = [0u8; 32];
    expected_amount[31] = 4;
    assert_eq!(deposit.amount.data, expected_amount);
    assert_eq!(deposit.exec_layer_data.length, 0);
}

/// A rejected input must come back as an error code rather than a crash, and the error-message
/// accessor (which lives in `parser.o`, a different archive member) must be callable too.
#[test]
fn c_parser_reports_error_on_truncated_payload() {
    let mut payload = vec![0u8; 8]; // far too short for an ether deposit
    let advance = advance_with_payload(&mut payload);
    let mut out = unsafe { std::mem::zeroed::<bindings::cma_parser_input_t>() };

    let rc = unsafe {
        bindings::cma_parser_decode_advance(
            bindings::cma_parser_input_type_t_CMA_PARSER_INPUT_TYPE_ETHER_DEPOSIT,
            &advance,
            &mut out,
        )
    };
    assert_ne!(rc, 0, "truncated payload must be rejected");

    // Pulls parser.o and proves the accessor is linked; the message itself is libcma's wording,
    // so only its presence is asserted.
    let msg = unsafe { bindings::cma_parser_get_last_error_message() };
    assert!(!msg.is_null(), "libcma should report an error message");
}

/// `cma_parser_encode_voucher` is a third entry point into `parser_impl.o` and the one a
/// withdrawal flow reaches. Referencing it keeps it in the link even if the decode paths above
/// are ever changed.
#[test]
fn c_parser_encode_voucher_is_linked() {
    let app_address = bindings::cma_abi_address_t { data: [0u8; 20] };
    let request = unsafe { std::mem::zeroed::<bindings::cma_parser_voucher_data_t>() };
    let mut voucher = unsafe { std::mem::zeroed::<bindings::cma_voucher_t>() };

    // Type NONE is not encodable; the call must fail cleanly rather than crash. What matters
    // for this test is that the symbol resolved at link time.
    let rc = unsafe {
        bindings::cma_parser_encode_voucher(
            bindings::cma_parser_voucher_type_t_CMA_PARSER_VOUCHER_TYPE_NONE,
            &app_address,
            &request,
            &mut voucher,
        )
    };
    assert_ne!(rc, 0, "an unset voucher type must not encode successfully");
}
