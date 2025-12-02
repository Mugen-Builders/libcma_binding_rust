use rust_parser::parser::{CmaParserInputData, CmaParserInputType, cma_decode_advance};
use rust_parser::helpers::{CARTESI_ADDRESSES, Portals, PortalMatcher};
use json::JsonValue;
use ethers::types::{Address, U256};

// Helper to create a basic JsonValue input structure
fn create_test_input(msg_sender: &str, payload: &str) -> JsonValue {
    let mut input = JsonValue::new_object();
    input["data"]["metadata"]["msg_sender"] = msg_sender.into();
    input["data"]["payload"] = payload.into();
    input
}

#[test]
fn test_ether_deposit_success() {
    let sender: Address = "0xf39fd6e51aad88f6f4ce6ab8827279cfffb92266".parse().unwrap();
    let amount = U256::from_dec_str("2000000000000000000").unwrap(); // 2 Ether in wei
    let payload = "0xf39fd6e51aad88f6f4ce6ab8827279cfffb922660000000000000000000000000000000000000000000000001bc16d674ec80000".to_string(); // Sample payload from the ether portal
    
    let input = create_test_input(CARTESI_ADDRESSES.get_portal_address(Portals::EtherPortal).unwrap(), &payload);
    
    match cma_decode_advance(input) {
        Ok(result) => {
            let is_correct_method = result.req_type == CmaParserInputType::CmaParserInputTypeEtherDeposit;
            let is_correct_sender = if let CmaParserInputData::EtherDeposit(deposit) = result.input {
                // println!("Deposit sender: {:?}, amount: {}, input_amount: {amount}, input_address: {sender}", deposit.sender, deposit.amount);
                if deposit.amount == amount && deposit.sender == sender {
                    true
                } else {
                    false
                }
                
            } else {
                false
            };

            assert_eq!(true, is_correct_method, "Expected Ether Deposit method");
            assert_eq!(true, is_correct_sender, "Expected correct sender and amount");

        }
        Err(e) => panic!("Expected success, got error: {:?}", e),
    }
}


#[test]
fn test_erc20_deposit_success() {
    let sender: Address = "0xf39fd6e51aad88f6f4ce6ab8827279cfffb92266".parse().unwrap();
    let amount = U256::from_dec_str("300000000000000000000").unwrap(); // 300 ERC20 tokens in wei
    let token_address: Address = "0xFBdB734EF6a23aD76863CbA6f10d0C5CBBD8342C".parse().unwrap(); // TEST token address
    let payload = "0xfbdb734ef6a23ad76863cba6f10d0c5cbbd8342cf39fd6e51aad88f6f4ce6ab8827279cfffb9226600000000000000000000000000000000000000000000001043561a8829300000".to_string(); // Sample payload from the ERC20 portal

    let input = create_test_input(CARTESI_ADDRESSES.get_portal_address(Portals::ERC20Portal).unwrap(), &payload);
    
    match cma_decode_advance(input) {
        Ok(result) => {
            let is_correct_method = result.req_type == CmaParserInputType::CmaParserInputTypeErc20Deposit;
            let is_correct_sender = if let CmaParserInputData::Erc20Deposit(deposit) = result.input {
                if deposit.amount == amount && deposit.sender == sender && deposit.token == token_address {
                    true
                } else {
                    false
                }
                
            } else {
                false
            };

            assert_eq!(true, is_correct_method, "Expected ERC20 Deposit method");
            assert_eq!(true, is_correct_sender, "Expected correct sender, token address and amount");

        }
        Err(e) => panic!("Expected success, got error: {:?}", e),
    }
}

#[test]
fn test_erc721_deposit_success() {
    let sender: Address = "0xf39fd6e51aad88f6f4ce6ab8827279cfffb92266".parse().unwrap();
    let token_id = U256::from_dec_str("1").unwrap(); // Sample token ID
    let token_address: Address = "0xBa46623aD94AB45850c4ecbA9555D26328917c3B".parse().unwrap(); // Sample ERC721 token address
    let payload = "0xba46623ad94ab45850c4ecba9555d26328917c3bf39fd6e51aad88f6f4ce6ab8827279cfffb9226600000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000040000000000000000000000000000000000000000000000000000000000000006000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000".to_string(); // Sample payload from the ERC721 portal 

    let input = create_test_input(CARTESI_ADDRESSES.get_portal_address(Portals::ERC721Portal).unwrap(), &payload);
    match cma_decode_advance(input) {
        Ok(result) => {
            let is_correct_method = result.req_type == CmaParserInputType::CmaParserInputTypeErc721Deposit;
            let is_correct_sender = if let CmaParserInputData::Erc721Deposit(deposit) = result.input {
                if deposit.token_id == token_id && deposit.sender == sender && deposit.token == token_address {
                    true
                } else {
                    false
                }
                
            } else {
                false
            };

            assert_eq!(true, is_correct_method, "Expected ERC721 Deposit method");
            assert_eq!(true, is_correct_sender, "Expected correct sender, token address and token ID");

        }
        Err(e) => panic!("Expected success, got error: {:?}", e),
    }
}


#[test]
fn test_ethers_withdrawal_success() {
    let recipient = "0xf39fd6e51aad88f6f4ce6ab8827279cfffb92266";
    let amount = U256::from_dec_str("1500000000000000000").unwrap(); // 1.5 Ether in wei
    let payload =   r#"{"function_type": "EtherWithdrawal", "amount": "1500000000000000000", "exec_layer_data": "0x"}"#.to_string();

    let payload_hex = hex::encode(payload.as_bytes());

    let input = create_test_input(recipient, &payload_hex);   

    match cma_decode_advance(input) {
        Ok(result) => {
            let is_correct_method = result.req_type == CmaParserInputType::CmaParserInputTypeEtherWithdrawal;
            let is_correct_recipient = if let CmaParserInputData::EtherWithdrawal(withdrawal) = result.input {
                if withdrawal.amount == amount && withdrawal.receiver == recipient.parse().unwrap() {
                    true
                } else {
                    false
                }
                
            } else {
                false
            };

            assert_eq!(true, is_correct_method, "Expected Ether Withdrawal method");
            assert_eq!(true, is_correct_recipient, "Expected correct recipient and amount");

        }
        Err(e) => panic!("Expected success, got error: {:?}", e),
    }
}

#[test]
fn test_erc20_withdrawal_success() {
    let recipient = "0xf39fd6e51aad88f6f4ce6ab8827279cfffb92266";
    let amount = U256::from_dec_str("50000000000000000000").unwrap(); // 50 ERC20 tokens in wei
    let token_address: Address = "0xFBdB734EF6a23aD76863CbA6f10d0C5CBBD8342C".parse().unwrap(); // TEST token address
    let payload =   r#"{"function_type": "Erc20Withdrawal", "token": "0xFBdB734EF6a23aD76863CbA6f10d0C5CBBD8342C", "amount": "50000000000000000000", "exec_layer_data": "0x"}"#.to_string();
    let payload_hex = hex::encode(payload.as_bytes());
    let input = create_test_input(recipient, &payload_hex);
    match cma_decode_advance(input) {
        Ok(result) => {
            let is_correct_method = result.req_type == CmaParserInputType::CmaParserInputTypeErc20Withdrawal;
            let is_correct_recipient = if let CmaParserInputData::Erc20Withdrawal(withdrawal) = result.input {
                if withdrawal.amount == amount && withdrawal.receiver == recipient.parse().unwrap() && withdrawal.token == token_address {
                    true
                } else {
                    false
                }
                
            } else {
                false
            };

            assert_eq!(true, is_correct_method, "Expected ERC20 Withdrawal method");
            assert_eq!(true, is_correct_recipient, "Expected correct recipient, token address and amount");

        }
        Err(e) => panic!("Expected success, got error: {:?}", e),
    }
}

#[test]
fn test_erc721_withdrawal_success() {
    let recipient = "0xf39fd6e51aad88f6f4ce6ab8827279cfffb92266";
    let token_id = U256::from_dec_str("1").unwrap(); // Sample token ID
    let token_address: Address = "0xBa46623aD94AB45850c4ecbA9555D26328917c3B".parse().unwrap(); // Sample ERC721 token address
    let payload =   r#"{"function_type": "Erc721Withdrawal", "token": "0xBa46623aD94AB45850c4ecbA9555D26328917c3B", "id": "1", "exec_layer_data": "0x"}"#.to_string();
    let payload_hex = hex::encode(payload.as_bytes());
    let input = create_test_input(recipient, &payload_hex);
    match cma_decode_advance(input) {
        Ok(result) => {
            let is_correct_method = result.req_type == CmaParserInputType::CmaParserInputTypeErc721Withdrawal;
            let is_correct_recipient = if let CmaParserInputData::Erc721Withdrawal(withdrawal) = result.input {
                if withdrawal.token_id == token_id && withdrawal.receiver == recipient.parse().unwrap() && withdrawal.token == token_address {
                    true
                } else {
                    false
                }
                
            } else {
                false
            };

            assert_eq!(true, is_correct_method, "Expected ERC721 Withdrawal method");
            assert_eq!(true, is_correct_recipient, "Expected correct recipient, token address and token ID");

        }
        Err(e) => panic!("Expected success, got error: {:?}", e),
    }
}

#[test]
fn test_ether_transfer_success() {
    let sender = "0xf39fd6e51aad88f6f4ce6ab8827279cfffb92266";
    let recipient: Address = "0x3e157927fb178490941bb18adcdc4144e442e32a".parse().unwrap();
    let amount = U256::from_dec_str("1500000000000000000").unwrap(); // 1.5 Ether in wei
    let payload =   r#"{"function_type": "EtherTransfer", "receiver": "0x3e157927fb178490941bb18adcdc4144e442e32a", "amount": "1500000000000000000", "exec_layer_data": "0x"}"#.to_string();

    let payload_hex = hex::encode(payload.as_bytes());
    let input = create_test_input(&sender.to_string(), &payload_hex);
    match cma_decode_advance(input) {
        Ok(result) => {
            let is_correct_method = result.req_type == CmaParserInputType::CmaParserInputTypeEtherTransfer;
            let is_correct_transfer = if let CmaParserInputData::EtherTransfer(transfer) = result.input {
                if transfer.amount == amount && transfer.sender == sender.parse().unwrap() && transfer.receiver == recipient {
                    true
                } else {
                    false
                }
                
            } else {
                false
            };

            assert_eq!(true, is_correct_method, "Expected Ether Transfer method");
            assert_eq!(true, is_correct_transfer, "Expected correct sender, recipient and amount");

        }
        Err(e) => panic!("Expected success, got error: {:?}", e),
    }

}

#[test]
fn test_erc20_transfer_success() {
    let sender = "0xf39fd6e51aad88f6f4ce6ab8827279cfffb92266";
    let recipient: Address = "0x3e157927fb178490941bb18adcdc4144e442e32a".parse().unwrap();
    let amount = U256::from_dec_str("50000000000000000000").unwrap(); // 50 ERC20 tokens in wei
    let token_address: Address = "0xFBdB734EF6a23aD76863CbA6f10d0C5CBBD8342C".parse().unwrap(); // TEST token address
    let payload =   r#"{"function_type": "Erc20Transfer", "token": "0xFBdB734EF6a23aD76863CbA6f10d0C5CBBD8342C", "receiver": "0x3e157927fb178490941bb18adcdc4144e442e32a", "amount": "50000000000000000000", "exec_layer_data": "0x"}"#.to_string();
    let payload_hex = hex::encode(payload.as_bytes());
    let input = create_test_input(&sender.to_string(), &payload_hex);
    match cma_decode_advance(input) {
        Ok(result) => {
            let is_correct_method = result.req_type == CmaParserInputType::CmaParserInputTypeErc20Transfer;
            let is_correct_transfer = if let CmaParserInputData::Erc20Transfer(transfer) = result.input {
                if transfer.amount == amount && transfer.sender == sender.parse().unwrap() && transfer.receiver == recipient && transfer.token == token_address {
                    true
                } else {
                    false
                }
            } else {
                false
            };
            assert_eq!(true, is_correct_method, "Expected ERC20 Transfer method");
            assert_eq!(true, is_correct_transfer, "Expected correct sender, recipient, token address and amount");
        }
        Err(e) => panic!("Expected success, got error: {:?}", e),
    }
}

#[test]
fn test_erc721_transfer_success() {
    let sender = "0xf39fd6e51aad88f6f4ce6ab8827279cfffb92266";
    let recipient: Address = "0x3e157927fb178490941bb18adcdc4144e442e32a".parse().unwrap();
    let token_id = U256::from_dec_str("1").unwrap(); // Sample token ID
    let token_address: Address = "0xBa46623aD94AB45850c4ecbA9555D26328917c3B".parse().unwrap(); // Sample ERC721 token address
    let payload =   r#"{"function_type": "Erc721Transfer", "token": "0xBa46623aD94AB45850c4ecbA9555D26328917c3B", "receiver": "0x3e157927fb178490941bb18adcdc4144e442e32a", "id": "1", "exec_layer_data": "0x"}"#.to_string();
    let payload_hex = hex::encode(payload.as_bytes());
    let input = create_test_input(&sender.to_string(), &payload_hex);
    match cma_decode_advance(input) {
        Ok(result) => {
            let is_correct_method = result.req_type == CmaParserInputType::CmaParserInputTypeErc721Transfer;
            let is_correct_transfer = if let CmaParserInputData::Erc721Transfer(transfer) = result.input {
                if transfer.token_id == token_id && transfer.sender == sender.parse().unwrap() && transfer.receiver == recipient && transfer.token == token_address {
                    true
                } else {
                    false
                }
            } else {
                false
            };
            assert_eq!(true, is_correct_method, "Expected ERC721 Transfer method");
            assert_eq!(true, is_correct_transfer, "Expected correct sender, recipient, token address and token ID");
        }
        Err(e) => panic!("Expected success, got error: {:?}", e),
    }
}

// #[test]
// fn test_ether_deposit_invalid_payload_length() {
//     // Payload too short

// }

// #[test]
// fn test_invalid_msg_sender() {

// }

// #[test]
// fn test_missing_payload() {

// }

// #[test]
// fn test_missing_msg_sender() {

// }