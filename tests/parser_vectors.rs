//! Canonical parser vectors ported from `third_party/machine-asset-tools/tests/parser.c`.

use ethers_core::types::{Address, U256};
use json::JsonValue;
use libcma_binding_rust::parser::{
    cma_decode_advance, cma_decode_inspect, cma_encode_voucher, CmaParserErc721VoucherFields,
    CmaParserEtherVoucherFields, CmaParserInputData, CmaParserInputType, CmaParserVoucherType,
    CmaVoucherFieldType,
};

fn advance_input(payload_hex: &str) -> JsonValue {
    let mut input = JsonValue::new_object();
    input["data"]["metadata"]["msg_sender"] = "0x0000000000000000000000000000000000000001".into();
    input["data"]["payload"] = payload_hex.into();
    input
}

fn inspect_input(payload_json: &str) -> JsonValue {
    let mut input = JsonValue::new_object();
    input["data"]["metadata"]["msg_sender"] = "0x0000000000000000000000000000000000000001".into();
    input["data"]["payload"] = hex::encode(payload_json.as_bytes()).into();
    input
}

#[test]
fn vector_ether_deposit_from_parser_c() {
    let payload = concat!(
        "0x",
        "0000000000000000000000000000000000000001",
        "0000000000000000000000000000000000000000000000000000000000000004"
    );
    let result = cma_decode_advance(
        CmaParserInputType::CmaParserInputTypeEtherDeposit,
        advance_input(payload),
    )
    .expect("ether deposit should decode");

    if let CmaParserInputData::EtherDeposit(deposit) = result.input {
        let mut expected_sender = [0u8; 20];
        expected_sender[19] = 1;
        assert_eq!(deposit.sender, Address::from(expected_sender));
        assert_eq!(deposit.amount, U256::from(4u64));
        assert!(deposit.exec_layer_data.is_empty());
    } else {
        panic!("expected ether deposit");
    }
}

#[test]
fn vector_erc20_deposit_from_parser_c() {
    let payload = concat!(
        "0x",
        "00000000000000000000000000000000000000ff",
        "0000000000000000000000000000000000000001",
        "0000000000000000000000000000000000000000000000000000000000000004"
    );
    let result = cma_decode_advance(
        CmaParserInputType::CmaParserInputTypeErc20Deposit,
        advance_input(payload),
    )
    .expect("erc20 deposit should decode");

    if let CmaParserInputData::Erc20Deposit(deposit) = result.input {
        let mut expected_token = [0u8; 20];
        expected_token[19] = 0xff;
        let mut expected_sender = [0u8; 20];
        expected_sender[19] = 1;
        assert_eq!(deposit.token, Address::from(expected_token));
        assert_eq!(deposit.sender, Address::from(expected_sender));
        assert_eq!(deposit.amount, U256::from(4u64));
    } else {
        panic!("expected erc20 deposit");
    }
}

#[test]
fn vector_ether_transfer_from_parser_c() {
    let payload = concat!(
        "0x",
        "ff67c903",
        "0000000000000000000000000000000000000000000000000000000000000002",
        "0000000000000000000000000000000000000000000000000000000000000011",
        "0000000000000000000000000000000000000000000000000000000000000060",
        "0000000000000000000000000000000000000000000000000000000000000000"
    );
    let result = cma_decode_advance(
        CmaParserInputType::CmaParserInputTypeAuto,
        advance_input(payload),
    )
    .expect("ether transfer should decode");

    if let CmaParserInputData::EtherTransfer(transfer) = result.input {
        assert_eq!(transfer.receiver, U256::from(2u64));
        assert_eq!(transfer.amount, U256::from(17u64));
    } else {
        panic!("expected ether transfer");
    }
}

#[test]
fn vector_ledger_get_balance_from_parser_c() {
    let payload = r#"{"method":"ledger_getBalance","params":["0x0000000000000000000000000000000000000001"]}"#;
    let result =
        cma_decode_inspect(inspect_input(payload)).expect("ledger_getBalance should decode");

    if let CmaParserInputData::Balance(balance) = result.input {
        assert_eq!(balance.account, U256::from(1u64));
    } else {
        panic!("expected balance inspect");
    }
}

#[test]
fn vector_erc721_voucher_from_parser_c() {
    let app_address = Address::from_slice(&[
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xbe, 0xef,
    ]);
    let receiver = Address::from_slice(&[
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2,
    ]);
    let token = Address::from_slice(&[
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xff,
    ]);

    let request = CmaVoucherFieldType::Erc721VoucherFields(CmaParserErc721VoucherFields {
        token,
        token_id: U256::from(1u64),
        receiver,
        application_address: app_address,
    });

    let voucher = cma_encode_voucher(
        CmaParserVoucherType::CmaParserVoucherTypeErc721,
        Some(app_address),
        request,
    )
    .expect("erc721 voucher should encode");

    assert!(voucher.payload.starts_with("0x42842e0e"));
    assert_eq!(voucher.value, "0x");
}

#[test]
fn vector_ether_voucher_value_from_parser_c() {
    let receiver: Address = "0x3e157927fb178490941bb18adcdc4144e442e32a"
        .parse()
        .unwrap();
    let amount = U256::from_dec_str("1500000000000000000").unwrap();
    let request = CmaVoucherFieldType::EtherVoucherFields(CmaParserEtherVoucherFields {
        receiver,
        amount,
    });

    let voucher = cma_encode_voucher(CmaParserVoucherType::CmaParserVoucherTypeEther, None, request)
        .expect("ether voucher should encode");

    let mut expected_value = [0u8; 32];
    amount.to_big_endian(&mut expected_value);
    assert_eq!(
        voucher.value,
        format!("0x{}", hex::encode(expected_value))
    );
    assert_eq!(voucher.payload, "0x");
}
