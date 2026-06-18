use crate::helpers::hex_to_string;
use ethers_core::abi::{ParamType, Token, decode, encode};
use ethers_core::types::{Address, Bytes, U256};
use ethers_core::utils::{id, to_checksum};

use hex;
use json::JsonValue;
use std::cell::RefCell;

thread_local! {
    static LAST_PARSER_ERROR: RefCell<String> = RefCell::new(String::new());
}

fn set_last_parser_error(message: impl Into<String>) {
    LAST_PARSER_ERROR.with(|msg| *msg.borrow_mut() = message.into());
}

fn payload_bytes(input: &JsonValue) -> Result<Vec<u8>, CmaParserError> {
    let payload_hex = input["data"]["payload"]
        .as_str()
        .ok_or(CmaParserError::MalformedInput)?;
    hex::decode(payload_hex.trim_start_matches("0x"))
        .map_err(|_| CmaParserError::MalformedInput)
}

fn require_len(bytes: &[u8], min: usize) -> Result<(), CmaParserError> {
    if bytes.len() < min {
        Err(CmaParserError::MalformedInput)
    } else {
        Ok(())
    }
}

fn decode_abi_tail_two_bytes(tail: &[u8]) -> Result<(Bytes, Bytes), CmaParserError> {
    if tail.is_empty() {
        return Ok((Bytes::from(vec![]), Bytes::from(vec![])));
    }
    let decoded = decode(
        &[ParamType::Bytes, ParamType::Bytes],
        tail,
    )
    .map_err(|_| CmaParserError::MalformedInput)?;
    let base = match &decoded[0] {
        Token::Bytes(b) => Bytes::from(b.clone()),
        _ => return Err(CmaParserError::MalformedInput),
    };
    let exec = match &decoded[1] {
        Token::Bytes(b) => Bytes::from(b.clone()),
        _ => return Err(CmaParserError::MalformedInput),
    };
    Ok((base, exec))
}

fn decode_abi_tail_batch(
    tail: &[u8],
) -> Result<(Vec<U256>, Vec<U256>, Bytes, Bytes), CmaParserError> {
    if tail.is_empty() {
        return Err(CmaParserError::MalformedInput);
    }
    let decoded = decode(
        &[
            ParamType::Array(Box::new(ParamType::Uint(256))),
            ParamType::Array(Box::new(ParamType::Uint(256))),
            ParamType::Bytes,
            ParamType::Bytes,
        ],
        tail,
    )
    .map_err(|_| CmaParserError::MalformedInput)?;

    let token_ids = match &decoded[0] {
        Token::Array(items) => items
            .iter()
            .map(|item| match item {
                Token::Uint(v) => Ok(*v),
                _ => Err(CmaParserError::MalformedInput),
            })
            .collect::<Result<Vec<_>, _>>()?,
        _ => return Err(CmaParserError::MalformedInput),
    };
    let amounts = match &decoded[1] {
        Token::Array(items) => items
            .iter()
            .map(|item| match item {
                Token::Uint(v) => Ok(*v),
                _ => Err(CmaParserError::MalformedInput),
            })
            .collect::<Result<Vec<_>, _>>()?,
        _ => return Err(CmaParserError::MalformedInput),
    };
    let base = match &decoded[2] {
        Token::Bytes(b) => Bytes::from(b.clone()),
        _ => return Err(CmaParserError::MalformedInput),
    };
    let exec = match &decoded[3] {
        Token::Bytes(b) => Bytes::from(b.clone()),
        _ => return Err(CmaParserError::MalformedInput),
    };

    if token_ids.is_empty() || amounts.is_empty() || token_ids.len() != amounts.len() {
        return Err(CmaParserError::MalformedInput);
    }

    Ok((token_ids, amounts, base, exec))
}

fn parse_hex_account_id(value: &str) -> Result<U256, CmaParserError> {
    let value = value.trim();
    if !value.starts_with("0x") {
        return Err(CmaParserError::MalformedInput);
    }
    let hex_body = &value[2..];
    if hex_body.len() % 2 != 0 {
        return Err(CmaParserError::MalformedInput);
    }

    let mut bytes = [0u8; 32];
    let offset = if value.len() == 42 {
        12
    } else if value.len() <= 66 {
        (64usize.saturating_sub(hex_body.len())) / 2
    } else {
        return Err(CmaParserError::MalformedInput);
    };

    let decoded = hex::decode(hex_body).map_err(|_| CmaParserError::MalformedInput)?;
    if offset + decoded.len() > 32 {
        return Err(CmaParserError::MalformedInput);
    }
    bytes[offset..offset + decoded.len()].copy_from_slice(&decoded);
    Ok(U256::from_big_endian(&bytes))
}

fn parse_hex_token_id(value: &str) -> Result<U256, CmaParserError> {
    let value = value.trim();
    if !value.starts_with("0x") {
        return Err(CmaParserError::MalformedInput);
    }
    let mut hex_body = value[2..].to_string();
    if hex_body.len() % 2 != 0 {
        hex_body.insert(0, '0');
    }
    if hex_body.len() > 64 {
        return Err(CmaParserError::MalformedInput);
    }
    let mut bytes = [0u8; 32];
    let offset = (64usize.saturating_sub(hex_body.len())) / 2;
    let decoded = hex::decode(hex_body).map_err(|_| CmaParserError::MalformedInput)?;
    if offset + decoded.len() > 32 {
        return Err(CmaParserError::MalformedInput);
    }
    bytes[offset..offset + decoded.len()].copy_from_slice(&decoded);
    Ok(U256::from_big_endian(&bytes))
}

fn parse_token_address(value: &str) -> Result<Address, CmaParserError> {
    if value.len() != 42 || !value.starts_with("0x") {
        return Err(CmaParserError::MalformedInput);
    }
    value
        .parse::<Address>()
        .map_err(|_| CmaParserError::MalformedInput)
}

fn tokenize_u256_list(tokens: &[Token]) -> Result<Vec<U256>, CmaParserError> {
    tokens
        .iter()
        .map(|item| match item {
            Token::Uint(v) => Ok(*v),
            _ => Err(CmaParserError::MalformedInput),
        })
        .collect()
}

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TxHexCodes {
    // Bytecode for solidity WithdrawEther(uint256,bytes) = 8cf70f0b
    WithdrawEther = 0x8cf70f0b,
    // Bytecode for solidity WithdrawErc20(address,uint256,bytes) = 4f94d342
    WithdrawErc20 = 0x4f94d342,
    // Bytecode for solidity WithdrawErc721(address,uint256,bytes) = 33acf293
    WithdrawErc721 = 0x33acf293,
    // Bytecode for solidity WithdrawErc1155Single(address,uint256,uint256,bytes) = 8bb0a811
    WithdrawErc1155Single = 0x8bb0a811,
    // Bytecode for solidity WithdrawErc1155Batch(address,uint256[],uint256[],bytes) = 50c80019
    WithdrawErc1155Batch = 0x50c80019,

    // Bytecode for solidity TransferEther(bytes32,uint256,bytes) = ff67c903
    TransferEther = 0xff67c903,
    // Bytecode for solidity TransferErc20(address,bytes32,uint256,bytes) = 03d61dcd
    TransferErc20 = 0x03d61dcd,
    // Bytecode for solidity TransferErc721(address,bytes32,uint256,bytes) = af615a5a
    TransferErc721 = 0xaf615a5a,
    // Bytecode for solidity TransferErc1155Single(address,bytes32,uint256,uint256,bytes) = e1c913ed
    TransferErc1155Single = 0xe1c913ed,
    // Bytecode for solidity TransferErc1155Batch(address,bytes32,uint256[],uint256[],bytes) = 638ac6f9
    TransferErc1155Batch = 0x638ac6f9,
    Unidentified
}

impl TxHexCodes {
    pub fn to_string(&self) -> &str {
        match self {
            Self::WithdrawEther => "0x8cf70f0b",
            Self::WithdrawErc20 => "0x4f94d342",
            Self::WithdrawErc721 => "0x33acf293",
            Self::WithdrawErc1155Single => "0x8bb0a811",
            Self::WithdrawErc1155Batch => "0x50c80019",
            Self::TransferEther => "0xff67c903",
            Self::TransferErc20 => "0x03d61dcd",
            Self::TransferErc721 => "0xaf615a5a",
            Self::TransferErc1155Single => "0xe1c913ed",
            Self::TransferErc1155Batch => "0x638ac6f9",
            Self::Unidentified => "0x00000000"
        }
    }

    pub fn from_string(input: &str) -> Self {
        match input {
            "0x8cf70f0b" => Self::WithdrawEther,
            "0x4f94d342" => Self::WithdrawErc20,
            "0x33acf293" => Self::WithdrawErc721,
            "0x8bb0a811" => Self::WithdrawErc1155Single,
            "0x50c80019" => Self::WithdrawErc1155Batch,
            "0xff67c903" => Self::TransferEther,
            "0x03d61dcd" => Self::TransferErc20,
            "0xaf615a5a" => Self::TransferErc721,
            "0xe1c913ed" => Self::TransferErc1155Single,
            "0x638ac6f9" => Self::TransferErc1155Batch,
            _ => Self::Unidentified, // default fallback
        }
    }

    pub fn to_input_type(&self) -> CmaParserInputType {
        match self {
            Self::WithdrawEther => CmaParserInputType::CmaParserInputTypeEtherWithdrawal,
            Self::WithdrawErc20 => CmaParserInputType::CmaParserInputTypeErc20Withdrawal,
            Self::WithdrawErc721 => CmaParserInputType::CmaParserInputTypeErc721Withdrawal,
            Self::WithdrawErc1155Single => CmaParserInputType::CmaParserInputTypeErc1155SingleWithdrawal,
            Self::WithdrawErc1155Batch => CmaParserInputType::CmaParserInputTypeErc1155BatchWithdrawal,
            Self::TransferEther => CmaParserInputType::CmaParserInputTypeEtherTransfer,
            Self::TransferErc20 => CmaParserInputType::CmaParserInputTypeErc20Transfer,
            Self::TransferErc721 => CmaParserInputType::CmaParserInputTypeErc721Transfer,
            Self::TransferErc1155Single => CmaParserInputType::CmaParserInputTypeErc1155SingleTransfer,
            Self::TransferErc1155Batch => CmaParserInputType::CmaParserInputTypeErc1155BatchTransfer,
            Self::Unidentified => CmaParserInputType::CmaParserInputTypeUnidentified,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CmaVoucherFieldType {
    EtherVoucherFields(CmaParserEtherVoucherFields),
    Erc20VoucherFields(CmaParserErc20VoucherFields),
    Erc721VoucherFields(CmaParserErc721VoucherFields),
    Erc1155SingleVoucherFields(CmaParserErc1155SingleVoucherFields),
    Erc1155BatchVoucherFields(CmaParserErc1155BatchVoucherFields),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CmaParserInputType {
    CmaParserInputTypeNone,
    CmaParserInputTypeAuto,
    CmaParserInputTypeUnidentified,
    CmaParserInputTypeEtherDeposit,
    CmaParserInputTypeErc20Deposit,
    CmaParserInputTypeErc721Deposit,
    CmaParserInputTypeErc1155SingleDeposit,
    CmaParserInputTypeErc1155BatchDeposit,
    CmaParserInputTypeEtherWithdrawal,
    CmaParserInputTypeErc20Withdrawal,
    CmaParserInputTypeErc721Withdrawal,
    CmaParserInputTypeErc1155SingleWithdrawal,
    CmaParserInputTypeErc1155BatchWithdrawal,
    CmaParserInputTypeEtherTransfer,
    CmaParserInputTypeErc20Transfer,
    CmaParserInputTypeErc721Transfer,
    CmaParserInputTypeErc1155SingleTransfer,
    CmaParserInputTypeErc1155BatchTransfer,
    CmaParserInputTypeBalance,
    CmaParserInputTypeSupply,
}

impl CmaParserInputType {
    pub fn from_string(s: &str) -> Self {
        match s {
            "EtherDeposit" => CmaParserInputType::CmaParserInputTypeEtherDeposit,
            "Erc20Deposit" => CmaParserInputType::CmaParserInputTypeErc20Deposit,
            "Erc721Deposit" => CmaParserInputType::CmaParserInputTypeErc721Deposit,
            "Erc1155SingleDeposit" => CmaParserInputType::CmaParserInputTypeErc1155SingleDeposit,
            "Erc1155BatchDeposit" => CmaParserInputType::CmaParserInputTypeErc1155BatchDeposit,
            "EtherWithdrawal" => CmaParserInputType::CmaParserInputTypeEtherWithdrawal,
            "Erc20Withdrawal" => CmaParserInputType::CmaParserInputTypeErc20Withdrawal,
            "Erc721Withdrawal" => CmaParserInputType::CmaParserInputTypeErc721Withdrawal,
            "Erc1155SingleWithdrawal" => {
                CmaParserInputType::CmaParserInputTypeErc1155SingleWithdrawal
            }
            "Erc1155BatchWithdrawal" => {
                CmaParserInputType::CmaParserInputTypeErc1155BatchWithdrawal
            }
            "EtherTransfer" => CmaParserInputType::CmaParserInputTypeEtherTransfer,
            "Erc20Transfer" => CmaParserInputType::CmaParserInputTypeErc20Transfer,
            "Erc721Transfer" => CmaParserInputType::CmaParserInputTypeErc721Transfer,
            "Erc1155SingleTransfer" => CmaParserInputType::CmaParserInputTypeErc1155SingleTransfer,
            "Erc1155BatchTransfer" => CmaParserInputType::CmaParserInputTypeErc1155BatchTransfer,
            "ledger_getBalance" | "ledgerGetBalance" => CmaParserInputType::CmaParserInputTypeBalance,
            "ledger_getTotalSupply" | "ledgerGetTotalSupply" => {
                CmaParserInputType::CmaParserInputTypeSupply
            }
            _ => CmaParserInputType::CmaParserInputTypeUnidentified,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CmaParserVoucherType {
    CmaParserVoucherTypeNone,
    CmaParserVoucherTypeEther,
    CmaParserVoucherTypeErc20,
    CmaParserVoucherTypeErc721,
    CmaParserVoucherTypeErc1155Single,
    CmaParserVoucherTypeErc1155Batch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CmaParserError {
    Success,
    Unknown,
    Exception,
    IncompatibleInput,
    MalformedInput,
    InvalidAmount,
    Message(String),
}

impl CmaParserError {
    pub fn from_code(code: i32) -> Self {
        match code {
            0 => Self::Success,
            -2001 => Self::Unknown,
            -2002 => Self::Exception,
            -2003 => Self::IncompatibleInput,
            -2004 => Self::MalformedInput,
            -2005 => Self::InvalidAmount,
            _ => Self::Unknown,
        }
    }

    pub fn to_code(self) -> i32 {
        match self {
            Self::Success => 0,
            Self::Unknown => -2001,
            Self::Exception => -2002,
            Self::IncompatibleInput => -2003,
            Self::MalformedInput => -2004,
            Self::InvalidAmount => -2005,
            Self::Message(_) => -2002,
        }
    }

    pub fn message(&self) -> String {
        match self {
            Self::Message(msg) => msg.clone(),
            other => format!("{:?}", other),
        }
    }
}

pub fn cma_parser_get_last_error_message() -> Option<String> {
    LAST_PARSER_ERROR.with(|msg| {
        let value = msg.borrow().clone();
        if value.is_empty() {
            None
        } else {
            Some(value)
        }
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CmaVoucher {
    pub destination: String,
    pub value: String,
    pub payload: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CmaParserUnidentifiedInput {
    pub abi_encoded_bytes: Vec<u8>,
    pub msg_sender: Address,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CmaParserEtherVoucherFields {
    pub amount: U256,
    pub receiver: Address,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CmaParserErc20VoucherFields {
    pub token: Address,
    pub receiver: Address,
    // pub value: U256,
    pub amount: U256,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CmaParserErc721VoucherFields {
    pub token: Address,
    pub token_id: U256,
    pub receiver: Address,
    // pub value: U256,
    pub application_address: Address,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CmaParserErc1155SingleVoucherFields {
    pub token: Address,
    pub receiver: Address,
    pub token_id: U256,
    pub amount: U256,
    pub exec_layer_data: Bytes,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CmaParserErc1155BatchVoucherFields {
    pub token: Address,
    pub receiver: Address,
    pub token_ids: Vec<U256>,
    pub amounts: Vec<U256>,
    pub exec_layer_data: Bytes,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CmaParserEtherDeposit {
    pub sender: Address,
    pub amount: U256,
    pub exec_layer_data: Bytes,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CmaParserErc20Deposit {
    pub sender: Address,
    pub token: Address,
    pub amount: U256,
    pub exec_layer_data: Bytes,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CmaParserErc721Deposit {
    pub sender: Address,
    pub token: Address,
    pub token_id: U256,
    pub base_layer_data: Bytes,
    pub exec_layer_data: Bytes,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CmaParserErc1155SingleDeposit {
    pub sender: Address,
    pub token: Address,
    pub token_id: U256,
    pub amount: U256,
    pub base_layer_data: Bytes,
    pub exec_layer_data: Bytes,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CmaParserErc1155BatchDeposit {
    pub sender: Address,
    pub token: Address,
    pub count: usize,
    pub token_ids: Vec<U256>,
    pub amounts: Vec<U256>,
    pub base_layer_data: Bytes,
    pub exec_layer_data: Bytes,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CmaParserEtherWithdrawal {
    pub receiver: Address,
    pub amount: U256,
    pub exec_layer_data: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CmaParserErc20Withdrawal {
    pub receiver: Address,
    pub token: Address,
    pub amount: U256,
    pub exec_layer_data: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CmaParserErc721Withdrawal {
    pub receiver: Address,
    pub token: Address,
    pub token_id: U256,
    pub exec_layer_data: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CmaParserErc1155SingleWithdrawal {
    pub receiver: Address,
    pub token: Address,
    pub token_id: U256,
    pub amount: U256,
    pub exec_layer_data: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CmaParserErc1155BatchWithdrawal {
    pub receiver: Address,
    pub token: Address,
    pub count: usize,
    pub token_ids: Vec<U256>,
    pub amounts: Vec<U256>,
    pub exec_layer_data: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CmaParserEtherTransfer {
    pub receiver: U256,
    pub amount: U256,
    pub exec_layer_data: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CmaParserErc20Transfer {
    pub receiver: U256,
    pub token: Address,
    pub amount: U256,
    pub exec_layer_data: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CmaParserErc721Transfer {
    pub receiver: U256,
    pub token: Address,
    pub token_id: U256,
    pub exec_layer_data: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CmaParserErc1155SingleTransfer {
    pub receiver: U256,
    pub token: Address,
    pub token_id: U256,
    pub amount: U256,
    pub exec_layer_data: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CmaParserErc1155BatchTransfer {
    pub receiver: U256,
    pub token: Address,
    pub count: usize,
    pub token_ids: Vec<U256>,
    pub amounts: Vec<U256>,
    pub exec_layer_data: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CmaParserBalance {
    pub account: U256,
    pub token: Address,
    pub token_id: U256,
    pub exec_layer_data: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CmaParserSupply {
    pub token: Address,
    pub token_id: U256,
    pub exec_layer_data: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CmaParserInputData {
    EtherDeposit(CmaParserEtherDeposit),
    Erc20Deposit(CmaParserErc20Deposit),
    Erc721Deposit(CmaParserErc721Deposit),
    Erc1155SingleDeposit(CmaParserErc1155SingleDeposit),
    Erc1155BatchDeposit(CmaParserErc1155BatchDeposit),
    EtherWithdrawal(CmaParserEtherWithdrawal),
    Erc20Withdrawal(CmaParserErc20Withdrawal),
    Erc721Withdrawal(CmaParserErc721Withdrawal),
    Erc1155SingleWithdrawal(CmaParserErc1155SingleWithdrawal),
    Erc1155BatchWithdrawal(CmaParserErc1155BatchWithdrawal),
    EtherTransfer(CmaParserEtherTransfer),
    Erc20Transfer(CmaParserErc20Transfer),
    Erc721Transfer(CmaParserErc721Transfer),
    Erc1155SingleTransfer(CmaParserErc1155SingleTransfer),
    Erc1155BatchTransfer(CmaParserErc1155BatchTransfer),
    Balance(CmaParserBalance),
    Supply(CmaParserSupply),
    Unidentified(CmaParserUnidentifiedInput),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CmaParserInput {
    pub req_type: CmaParserInputType,
    pub input: CmaParserInputData,
}

fn handle_unidentified_method(input: JsonValue) -> Result<CmaParserInputData, CmaParserError> {
    let bytes = payload_bytes(&input)?;
    let msg_sender = input["data"]["metadata"]["msg_sender"]
        .as_str()
        .ok_or(CmaParserError::MalformedInput)?;
    let sender = Address::from_slice(
        &hex::decode(msg_sender.trim_start_matches("0x"))
            .map_err(|_| CmaParserError::MalformedInput)?,
    );

    Ok(CmaParserInputData::Unidentified(CmaParserUnidentifiedInput {
        abi_encoded_bytes: bytes,
        msg_sender: sender,
    }))
}

fn handle_parse_ether_deposit(input: JsonValue) -> Result<CmaParserInputData, CmaParserError> {
    let bytes = payload_bytes(&input)?;
    require_len(&bytes, 52)?;

    let sender = Address::from_slice(&bytes[0..20]);
    let value = U256::from_big_endian(&bytes[20..52]);
    let exec_layer_data = Bytes::from(bytes[52..].to_vec());

    Ok(CmaParserInputData::EtherDeposit(CmaParserEtherDeposit {
        sender,
        amount: value,
        exec_layer_data,
    }))
}

fn handle_parse_erc20_deposit(input: JsonValue) -> Result<CmaParserInputData, CmaParserError> {
    let bytes = payload_bytes(&input)?;
    require_len(&bytes, 72)?;

    let token = Address::from_slice(&bytes[0..20]);
    let sender = Address::from_slice(&bytes[20..40]);
    let amount = U256::from_big_endian(&bytes[40..72]);
    let exec_layer_data = Bytes::from(bytes[72..].to_vec());

    Ok(CmaParserInputData::Erc20Deposit(CmaParserErc20Deposit {
        sender,
        token,
        amount,
        exec_layer_data,
    }))
}

fn handle_parse_erc721_deposit(input: JsonValue) -> Result<CmaParserInputData, CmaParserError> {
    let bytes = payload_bytes(&input)?;
    require_len(&bytes, 72)?;

    let token = Address::from_slice(&bytes[0..20]);
    let sender = Address::from_slice(&bytes[20..40]);
    let token_id = U256::from_big_endian(&bytes[40..72]);
    let (base_layer_data, exec_layer_data) = decode_abi_tail_two_bytes(&bytes[72..])?;

    Ok(CmaParserInputData::Erc721Deposit(CmaParserErc721Deposit {
        sender,
        token,
        token_id,
        base_layer_data,
        exec_layer_data,
    }))
}


fn handle_parse_erc1155_single_deposit(
    input: JsonValue,
) -> Result<CmaParserInputData, CmaParserError> {
    let bytes = payload_bytes(&input)?;
    require_len(&bytes, 104)?;

    let token = Address::from_slice(&bytes[0..20]);
    let sender = Address::from_slice(&bytes[20..40]);
    let token_id = U256::from_big_endian(&bytes[40..72]);
    let amount = U256::from_big_endian(&bytes[72..104]);
    let (base_layer_data, exec_layer_data) = decode_abi_tail_two_bytes(&bytes[104..])?;

    Ok(CmaParserInputData::Erc1155SingleDeposit(
        CmaParserErc1155SingleDeposit {
            sender,
            token,
            token_id,
            amount,
            base_layer_data,
            exec_layer_data,
        },
    ))
}

fn handle_parse_erc1155_batch_deposit(
    input: JsonValue,
) -> Result<CmaParserInputData, CmaParserError> {
    let bytes = payload_bytes(&input)?;
    require_len(&bytes, 40)?;

    let token = Address::from_slice(&bytes[0..20]);
    let sender = Address::from_slice(&bytes[20..40]);
    let (token_ids, amounts, base_layer_data, exec_layer_data) =
        decode_abi_tail_batch(&bytes[40..])?;

    Ok(CmaParserInputData::Erc1155BatchDeposit(
        CmaParserErc1155BatchDeposit {
            sender,
            token,
            count: token_ids.len(),
            token_ids,
            amounts,
            base_layer_data,
            exec_layer_data,
        },
    ))
}

fn withdrawal_receiver(input: &JsonValue) -> Result<Address, CmaParserError> {
    let msg_sender = input["data"]["metadata"]["msg_sender"]
        .as_str()
        .ok_or(CmaParserError::MalformedInput)?;
    msg_sender
        .parse::<Address>()
        .map_err(|_| CmaParserError::MalformedInput)
}

fn decode_after_selector(bytes: &[u8]) -> Result<&[u8], CmaParserError> {
    require_len(bytes, 4)?;
    Ok(&bytes[4..])
}

fn exec_layer_hex(exec_layer_byte: &[u8]) -> String {
    format!("0x{}", hex::encode(exec_layer_byte))
}

fn handle_ether_withdrawal(input: JsonValue) -> Result<CmaParserInputData, CmaParserError> {
    let bytes = payload_bytes(&input)?;
    let encoded_args = decode_after_selector(&bytes)?;
    let receiver = withdrawal_receiver(&input)?;

    let decoded = decode(
        &[ParamType::Uint(256), ParamType::Bytes],
        encoded_args,
    )
    .map_err(|_| CmaParserError::MalformedInput)?;

    let amount = match &decoded[0] {
        Token::Uint(v) => *v,
        _ => return Err(CmaParserError::MalformedInput),
    };
    let exec_layer_byte = match &decoded[1] {
        Token::Bytes(b) => b.as_slice(),
        _ => return Err(CmaParserError::MalformedInput),
    };

    Ok(CmaParserInputData::EtherWithdrawal(CmaParserEtherWithdrawal {
        receiver,
        amount,
        exec_layer_data: exec_layer_hex(exec_layer_byte),
    }))
}

fn handle_erc20_withdrawal(input: JsonValue) -> Result<CmaParserInputData, CmaParserError> {
    let bytes = payload_bytes(&input)?;
    let encoded_args = decode_after_selector(&bytes)?;
    let receiver = withdrawal_receiver(&input)?;

    let decoded = decode(
        &[ParamType::Address, ParamType::Uint(256), ParamType::Bytes],
        encoded_args,
    )
    .map_err(|_| CmaParserError::MalformedInput)?;

    let token = match &decoded[0] {
        Token::Address(v) => *v,
        _ => return Err(CmaParserError::MalformedInput),
    };
    let amount = match &decoded[1] {
        Token::Uint(v) => *v,
        _ => return Err(CmaParserError::MalformedInput),
    };
    let exec_layer_byte = match &decoded[2] {
        Token::Bytes(b) => b.as_slice(),
        _ => return Err(CmaParserError::MalformedInput),
    };

    Ok(CmaParserInputData::Erc20Withdrawal(CmaParserErc20Withdrawal {
        receiver,
        token,
        amount,
        exec_layer_data: exec_layer_hex(exec_layer_byte),
    }))
}

fn handle_erc721_withdrawal(input: JsonValue) -> Result<CmaParserInputData, CmaParserError> {
    let bytes = payload_bytes(&input)?;
    let encoded_args = decode_after_selector(&bytes)?;
    let receiver = withdrawal_receiver(&input)?;

    let decoded = decode(
        &[ParamType::Address, ParamType::Uint(256), ParamType::Bytes],
        encoded_args,
    )
    .map_err(|_| CmaParserError::MalformedInput)?;

    let token = match &decoded[0] {
        Token::Address(v) => *v,
        _ => return Err(CmaParserError::MalformedInput),
    };
    let token_id = match &decoded[1] {
        Token::Uint(v) => *v,
        _ => return Err(CmaParserError::MalformedInput),
    };
    let exec_layer_byte = match &decoded[2] {
        Token::Bytes(b) => b.as_slice(),
        _ => return Err(CmaParserError::MalformedInput),
    };

    Ok(CmaParserInputData::Erc721Withdrawal(CmaParserErc721Withdrawal {
        receiver,
        token,
        token_id,
        exec_layer_data: exec_layer_hex(exec_layer_byte),
    }))
}

fn handle_erc1155_single_withdrawal(
    input: JsonValue,
) -> Result<CmaParserInputData, CmaParserError> {
    let bytes = payload_bytes(&input)?;
    let encoded_args = decode_after_selector(&bytes)?;
    let receiver = withdrawal_receiver(&input)?;

    let decoded = decode(
        &[
            ParamType::Address,
            ParamType::Uint(256),
            ParamType::Uint(256),
            ParamType::Bytes,
        ],
        encoded_args,
    )
    .map_err(|_| CmaParserError::MalformedInput)?;

    let token = match &decoded[0] {
        Token::Address(v) => *v,
        _ => return Err(CmaParserError::MalformedInput),
    };
    let token_id = match &decoded[1] {
        Token::Uint(v) => *v,
        _ => return Err(CmaParserError::MalformedInput),
    };
    let amount = match &decoded[2] {
        Token::Uint(v) => *v,
        _ => return Err(CmaParserError::MalformedInput),
    };
    let exec_layer_byte = match &decoded[3] {
        Token::Bytes(b) => b.as_slice(),
        _ => return Err(CmaParserError::MalformedInput),
    };

    Ok(CmaParserInputData::Erc1155SingleWithdrawal(
        CmaParserErc1155SingleWithdrawal {
            receiver,
            token,
            token_id,
            amount,
            exec_layer_data: exec_layer_hex(exec_layer_byte),
        },
    ))
}

fn handle_erc1155_batch_withdrawal(
    input: JsonValue,
) -> Result<CmaParserInputData, CmaParserError> {
    let bytes = payload_bytes(&input)?;
    let encoded_args = decode_after_selector(&bytes)?;
    let receiver = withdrawal_receiver(&input)?;

    let decoded = decode(
        &[
            ParamType::Address,
            ParamType::Array(Box::new(ParamType::Uint(256))),
            ParamType::Array(Box::new(ParamType::Uint(256))),
            ParamType::Bytes,
        ],
        encoded_args,
    )
    .map_err(|_| CmaParserError::MalformedInput)?;

    let token = match &decoded[0] {
        Token::Address(v) => *v,
        _ => return Err(CmaParserError::MalformedInput),
    };
    let token_ids = match &decoded[1] {
        Token::Array(items) => tokenize_u256_list(items)?,
        _ => return Err(CmaParserError::MalformedInput),
    };
    let amounts = match &decoded[2] {
        Token::Array(items) => tokenize_u256_list(items)?,
        _ => return Err(CmaParserError::MalformedInput),
    };
    let exec_layer_byte = match &decoded[3] {
        Token::Bytes(b) => b.as_slice(),
        _ => return Err(CmaParserError::MalformedInput),
    };

    if token_ids.len() != amounts.len() || token_ids.is_empty() {
        return Err(CmaParserError::MalformedInput);
    }

    Ok(CmaParserInputData::Erc1155BatchWithdrawal(
        CmaParserErc1155BatchWithdrawal {
            receiver,
            token,
            count: token_ids.len(),
            token_ids,
            amounts,
            exec_layer_data: exec_layer_hex(exec_layer_byte),
        },
    ))
}

fn handle_ether_transfer(input: JsonValue) -> Result<CmaParserInputData, CmaParserError> {
    let bytes = payload_bytes(&input)?;
    let encoded_args = decode_after_selector(&bytes)?;

    let decoded = decode(
        &[ParamType::Uint(256), ParamType::Uint(256), ParamType::Bytes],
        encoded_args,
    )
    .map_err(|_| CmaParserError::MalformedInput)?;

    let receiver = match &decoded[0] {
        Token::Uint(v) => *v,
        _ => return Err(CmaParserError::MalformedInput),
    };
    let amount = match &decoded[1] {
        Token::Uint(v) => *v,
        _ => return Err(CmaParserError::MalformedInput),
    };
    let exec_layer_byte = match &decoded[2] {
        Token::Bytes(b) => b.as_slice(),
        _ => return Err(CmaParserError::MalformedInput),
    };

    Ok(CmaParserInputData::EtherTransfer(CmaParserEtherTransfer {
        receiver,
        amount,
        exec_layer_data: exec_layer_hex(exec_layer_byte),
    }))
}

fn handle_erc20_transfer(input: JsonValue) -> Result<CmaParserInputData, CmaParserError> {
    let bytes = payload_bytes(&input)?;
    let encoded_args = decode_after_selector(&bytes)?;

    let decoded = decode(
        &[
            ParamType::Address,
            ParamType::Uint(256),
            ParamType::Uint(256),
            ParamType::Bytes,
        ],
        encoded_args,
    )
    .map_err(|_| CmaParserError::MalformedInput)?;

    let token = match &decoded[0] {
        Token::Address(v) => *v,
        _ => return Err(CmaParserError::MalformedInput),
    };
    let receiver = match &decoded[1] {
        Token::Uint(v) => *v,
        _ => return Err(CmaParserError::MalformedInput),
    };
    let amount = match &decoded[2] {
        Token::Uint(v) => *v,
        _ => return Err(CmaParserError::MalformedInput),
    };
    let exec_layer_byte = match &decoded[3] {
        Token::Bytes(b) => b.as_slice(),
        _ => return Err(CmaParserError::MalformedInput),
    };

    Ok(CmaParserInputData::Erc20Transfer(CmaParserErc20Transfer {
        receiver,
        token,
        amount,
        exec_layer_data: exec_layer_hex(exec_layer_byte),
    }))
}

fn handle_erc721_transfer(input: JsonValue) -> Result<CmaParserInputData, CmaParserError> {
    let bytes = payload_bytes(&input)?;
    let encoded_args = decode_after_selector(&bytes)?;

    let decoded = decode(
        &[
            ParamType::Address,
            ParamType::Uint(256),
            ParamType::Uint(256),
            ParamType::Bytes,
        ],
        encoded_args,
    )
    .map_err(|_| CmaParserError::MalformedInput)?;

    let token = match &decoded[0] {
        Token::Address(v) => *v,
        _ => return Err(CmaParserError::MalformedInput),
    };
    let receiver = match &decoded[1] {
        Token::Uint(v) => *v,
        _ => return Err(CmaParserError::MalformedInput),
    };
    let token_id = match &decoded[2] {
        Token::Uint(v) => *v,
        _ => return Err(CmaParserError::MalformedInput),
    };
    let exec_layer_byte = match &decoded[3] {
        Token::Bytes(b) => b.as_slice(),
        _ => return Err(CmaParserError::MalformedInput),
    };

    Ok(CmaParserInputData::Erc721Transfer(CmaParserErc721Transfer {
        receiver,
        token,
        token_id,
        exec_layer_data: exec_layer_hex(exec_layer_byte),
    }))
}

fn handle_erc1155_single_transfer(
    input: JsonValue,
) -> Result<CmaParserInputData, CmaParserError> {
    let bytes = payload_bytes(&input)?;
    let encoded_args = decode_after_selector(&bytes)?;

    let decoded = decode(
        &[
            ParamType::Address,
            ParamType::Uint(256),
            ParamType::Uint(256),
            ParamType::Uint(256),
            ParamType::Bytes,
        ],
        encoded_args,
    )
    .map_err(|_| CmaParserError::MalformedInput)?;

    let token = match &decoded[0] {
        Token::Address(v) => *v,
        _ => return Err(CmaParserError::MalformedInput),
    };
    let receiver = match &decoded[1] {
        Token::Uint(v) => *v,
        _ => return Err(CmaParserError::MalformedInput),
    };
    let token_id = match &decoded[2] {
        Token::Uint(v) => *v,
        _ => return Err(CmaParserError::MalformedInput),
    };
    let amount = match &decoded[3] {
        Token::Uint(v) => *v,
        _ => return Err(CmaParserError::MalformedInput),
    };
    let exec_layer_byte = match &decoded[4] {
        Token::Bytes(b) => b.as_slice(),
        _ => return Err(CmaParserError::MalformedInput),
    };

    Ok(CmaParserInputData::Erc1155SingleTransfer(
        CmaParserErc1155SingleTransfer {
            receiver,
            token,
            token_id,
            amount,
            exec_layer_data: exec_layer_hex(exec_layer_byte),
        },
    ))
}

fn handle_erc1155_batch_transfer(
    input: JsonValue,
) -> Result<CmaParserInputData, CmaParserError> {
    let bytes = payload_bytes(&input)?;
    let encoded_args = decode_after_selector(&bytes)?;

    let decoded = decode(
        &[
            ParamType::Address,
            ParamType::Uint(256),
            ParamType::Array(Box::new(ParamType::Uint(256))),
            ParamType::Array(Box::new(ParamType::Uint(256))),
            ParamType::Bytes,
        ],
        encoded_args,
    )
    .map_err(|_| CmaParserError::MalformedInput)?;

    let token = match &decoded[0] {
        Token::Address(v) => *v,
        _ => return Err(CmaParserError::MalformedInput),
    };
    let receiver = match &decoded[1] {
        Token::Uint(v) => *v,
        _ => return Err(CmaParserError::MalformedInput),
    };
    let token_ids = match &decoded[2] {
        Token::Array(items) => tokenize_u256_list(items)?,
        _ => return Err(CmaParserError::MalformedInput),
    };
    let amounts = match &decoded[3] {
        Token::Array(items) => tokenize_u256_list(items)?,
        _ => return Err(CmaParserError::MalformedInput),
    };
    let exec_layer_byte = match &decoded[4] {
        Token::Bytes(b) => b.as_slice(),
        _ => return Err(CmaParserError::MalformedInput),
    };

    if token_ids.len() != amounts.len() || token_ids.is_empty() {
        return Err(CmaParserError::MalformedInput);
    }

    Ok(CmaParserInputData::Erc1155BatchTransfer(
        CmaParserErc1155BatchTransfer {
            receiver,
            token,
            count: token_ids.len(),
            token_ids,
            amounts,
            exec_layer_data: exec_layer_hex(exec_layer_byte),
        },
    ))
}

pub fn cma_decode_advance(
    req_type: CmaParserInputType,
    input: JsonValue,
) -> Result<CmaParserInput, CmaParserError> {
    let result = cma_decode_advance_inner(req_type, input);
    if let Err(ref err) = result {
        set_last_parser_error(err.message());
    }
    result
}

fn cma_decode_advance_inner(
    req_type: CmaParserInputType,
    input: JsonValue,
) -> Result<CmaParserInput, CmaParserError> {
    // Determine the portal type based on msg_sender
    match req_type {
        CmaParserInputType::CmaParserInputTypeErc1155BatchDeposit => {
            return handle_parse_erc1155_batch_deposit(input).map(|data| CmaParserInput {
                req_type,
                input: data,
            });
        }
        CmaParserInputType::CmaParserInputTypeErc1155SingleDeposit=> {
            return handle_parse_erc1155_single_deposit(input).map(|data| CmaParserInput {
                req_type,
                input: data,
            });
        }
        CmaParserInputType::CmaParserInputTypeErc721Deposit => {
            return handle_parse_erc721_deposit(input).map(|data| CmaParserInput {
                req_type,
                input: data,
            });
        }
        CmaParserInputType::CmaParserInputTypeErc20Deposit => {
            return handle_parse_erc20_deposit(input).map(|data| CmaParserInput {
                req_type,
                input: data,
            });
        }
        CmaParserInputType::CmaParserInputTypeEtherDeposit => {
            return handle_parse_ether_deposit(input).map(|data| CmaParserInput {
                req_type,
                input: data,
            });
        }
        // IF CALLER IS NOT ANY OF THE ABOVE PORTALS, WE TRY TO PARSE THE PAYLOAD FOR WITHDRAWALS/TRANSFERS
        CmaParserInputType::CmaParserInputTypeAuto => {
            let bytes = payload_bytes(&input)?;
            require_len(&bytes, 4)?;
            let selector_str = format!("0x{}", hex::encode(&bytes[..4]));
            let req_type = TxHexCodes::from_string(&selector_str).to_input_type();

            let result = match req_type {
                CmaParserInputType::CmaParserInputTypeEtherWithdrawal => {
                    handle_ether_withdrawal(input)
                }
                CmaParserInputType::CmaParserInputTypeErc20Withdrawal => {
                    handle_erc20_withdrawal(input)
                }
                CmaParserInputType::CmaParserInputTypeErc721Withdrawal => {
                    handle_erc721_withdrawal(input)
                }
                CmaParserInputType::CmaParserInputTypeErc1155SingleWithdrawal => {
                    handle_erc1155_single_withdrawal(input)
                }
                CmaParserInputType::CmaParserInputTypeErc1155BatchWithdrawal => {
                    handle_erc1155_batch_withdrawal(input)
                }
                CmaParserInputType::CmaParserInputTypeEtherTransfer => {
                    handle_ether_transfer(input)
                }
                CmaParserInputType::CmaParserInputTypeErc20Transfer => {
                    handle_erc20_transfer(input)
                }
                CmaParserInputType::CmaParserInputTypeErc721Transfer => {
                    handle_erc721_transfer(input)
                }
                CmaParserInputType::CmaParserInputTypeErc1155SingleTransfer => {
                    handle_erc1155_single_transfer(input)
                }
                CmaParserInputType::CmaParserInputTypeErc1155BatchTransfer => {
                    handle_erc1155_batch_transfer(input)
                }
                CmaParserInputType::CmaParserInputTypeUnidentified => {
                    handle_unidentified_method(input)
                }
                _ => Err(CmaParserError::IncompatibleInput),
            };

            result.map(|data| CmaParserInput { req_type, input: data })
        }
        _ => Err(CmaParserError::Unknown),
    }
}

fn inspect_param_strings(parsed_json: &JsonValue) -> Result<Vec<String>, CmaParserError> {
    let params_val = &parsed_json["params"];
    if !params_val.is_array() {
        return Err(CmaParserError::MalformedInput);
    }
    params_val
        .members()
        .map(|member| {
            member
                .as_str()
                .map(str::to_string)
                .ok_or(CmaParserError::MalformedInput)
        })
        .collect()
}

fn handle_ledger_get_balance(parsed_json: JsonValue) -> Result<CmaParserBalance, CmaParserError> {
    let params = inspect_param_strings(&parsed_json)?;
    if params.is_empty() || params.len() > 4 {
        return Err(CmaParserError::MalformedInput);
    }

    let account = parse_hex_account_id(&params[0])?;
    let mut balance = CmaParserBalance {
        account,
        token: Address::zero(),
        token_id: U256::zero(),
        exec_layer_data: String::new(),
    };

    if params.len() >= 2 {
        balance.token = parse_token_address(&params[1])?;
    }
    if params.len() >= 3 {
        balance.token_id = parse_hex_token_id(&params[2])?;
    }
    if params.len() == 4 {
        balance.exec_layer_data = params[3].clone();
    }

    Ok(balance)
}

fn handle_ledger_get_supply(parsed_json: JsonValue) -> Result<CmaParserInputData, CmaParserError> {
    let params = if parsed_json["params"].is_array() {
        inspect_param_strings(&parsed_json)?
    } else {
        Vec::new()
    };

    if params.len() > 3 {
        return Err(CmaParserError::MalformedInput);
    }

    let mut supply = CmaParserSupply {
        token: Address::zero(),
        token_id: U256::zero(),
        exec_layer_data: String::new(),
    };

    if !params.is_empty() {
        supply.token = parse_token_address(&params[0])?;
    }
    if params.len() >= 2 {
        supply.token_id = parse_hex_token_id(&params[1])?;
    }
    if params.len() == 3 {
        supply.exec_layer_data = params[2].clone();
    }

    Ok(CmaParserInputData::Supply(supply))
}

pub fn cma_decode_inspect(input: JsonValue) -> Result<CmaParserInput, CmaParserError> {
    let result = cma_decode_inspect_inner(input);
    if let Err(ref err) = result {
        set_last_parser_error(err.message());
    }
    result
}

fn cma_decode_inspect_inner(input: JsonValue) -> Result<CmaParserInput, CmaParserError> {
    let payload_hex = input["data"]["payload"]
        .as_str()
        .ok_or(CmaParserError::MalformedInput)?;
    let payload_str = hex_to_string(payload_hex)
        .map_err(|_| CmaParserError::MalformedInput)?;
    let payload_json = json::parse(&payload_str)
        .map_err(|_| CmaParserError::MalformedInput)?;

    let method = payload_json["method"]
        .as_str()
        .ok_or(CmaParserError::MalformedInput)?;
    let req_type = CmaParserInputType::from_string(method);
    if req_type == CmaParserInputType::CmaParserInputTypeUnidentified {
        return Err(CmaParserError::IncompatibleInput);
    }

    match req_type {
        CmaParserInputType::CmaParserInputTypeBalance => {
            handle_ledger_get_balance(payload_json).map(|data| CmaParserInput {
                req_type,
                input: CmaParserInputData::Balance(data),
            })
        }
        CmaParserInputType::CmaParserInputTypeSupply => {
            handle_ledger_get_supply(payload_json).map(|data| CmaParserInput {
                req_type,
                input: data,
            })
        }
        _ => Err(CmaParserError::IncompatibleInput),
    }
}

fn handle_ether_voucher_encoding(
    voucher_request: &CmaVoucherFieldType,
) -> Result<CmaVoucher, CmaParserError> {
    if let CmaVoucherFieldType::EtherVoucherFields(fields) = voucher_request {
        let mut value_bytes = [0u8; 32];
        fields.amount.to_big_endian(&mut value_bytes);

        Ok(CmaVoucher {
            destination: to_checksum(&fields.receiver, None),
            value: format!("0x{}", hex::encode(value_bytes)),
            payload: "0x".to_string(),
        })
    } else {
        Err(CmaParserError::IncompatibleInput)
    }
}

fn handle_erc20_voucher_encoding(
    voucher_request: &CmaVoucherFieldType,
) -> Result<CmaVoucher, CmaParserError> {
    if let CmaVoucherFieldType::Erc20VoucherFields(fields) = voucher_request {
        let args = vec![Token::Address(fields.receiver), Token::Uint(fields.amount)];
        let selector = &id("transfer(address,uint256)")[..4];
        let mut payload_bytes = Vec::new();
        payload_bytes.extend_from_slice(selector);
        payload_bytes.extend_from_slice(&encode(&args));

        Ok(CmaVoucher {
            destination: to_checksum(&fields.token, None),
            value: "0x".to_string(),
            payload: format!("0x{}", hex::encode(payload_bytes)),
        })
    } else {
        Err(CmaParserError::IncompatibleInput)
    }
}

fn handle_erc721_voucher_encoding(
    app_address: Address,
    voucher_request: &CmaVoucherFieldType,
) -> Result<CmaVoucher, CmaParserError> {
    if let CmaVoucherFieldType::Erc721VoucherFields(fields) = voucher_request {
        let args = vec![
            Token::Address(app_address),
            Token::Address(fields.receiver),
            Token::Uint(fields.token_id),
        ];
        let selector = &id("safeTransferFrom(address,address,uint256)")[..4];
        let mut payload_bytes = Vec::new();
        payload_bytes.extend_from_slice(selector);
        payload_bytes.extend_from_slice(&encode(&args));

        Ok(CmaVoucher {
            destination: to_checksum(&fields.token, None),
            value: "0x".to_string(),
            payload: format!("0x{}", hex::encode(payload_bytes)),
        })
    } else {
        Err(CmaParserError::IncompatibleInput)
    }
}

fn handle_erc1155_single_voucher_encoding(
    app_address: Address,
    voucher_request: &CmaVoucherFieldType,
) -> Result<CmaVoucher, CmaParserError> {
    if let CmaVoucherFieldType::Erc1155SingleVoucherFields(fields) = voucher_request {
        let args = vec![
            Token::Address(app_address),
            Token::Address(fields.receiver),
            Token::Uint(fields.token_id),
            Token::Uint(fields.amount),
            Token::Bytes(fields.exec_layer_data.to_vec()),
        ];
        let selector = &id("safeTransferFrom(address,address,uint256,uint256,bytes)")[..4];
        let mut payload_bytes = Vec::new();
        payload_bytes.extend_from_slice(selector);
        payload_bytes.extend_from_slice(&encode(&args));

        Ok(CmaVoucher {
            destination: to_checksum(&fields.token, None),
            value: "0x".to_string(),
            payload: format!("0x{}", hex::encode(payload_bytes)),
        })
    } else {
        Err(CmaParserError::IncompatibleInput)
    }
}

fn handle_erc1155_batch_voucher_encoding(
    app_address: Address,
    voucher_request: &CmaVoucherFieldType,
) -> Result<CmaVoucher, CmaParserError> {
    if let CmaVoucherFieldType::Erc1155BatchVoucherFields(fields) = voucher_request {
        if fields.token_ids.len() != fields.amounts.len() || fields.token_ids.is_empty() {
            return Err(CmaParserError::MalformedInput);
        }

        let token_id_tokens: Vec<Token> = fields
            .token_ids
            .iter()
            .map(|id| Token::Uint(*id))
            .collect();
        let amount_tokens: Vec<Token> = fields
            .amounts
            .iter()
            .map(|amount| Token::Uint(*amount))
            .collect();

        let args = vec![
            Token::Address(app_address),
            Token::Address(fields.receiver),
            Token::Array(token_id_tokens),
            Token::Array(amount_tokens),
            Token::Bytes(fields.exec_layer_data.to_vec()),
        ];
        let selector = &id("safeBatchTransferFrom(address,address,uint256[],uint256[],bytes)")[..4];
        let mut payload_bytes = Vec::new();
        payload_bytes.extend_from_slice(selector);
        payload_bytes.extend_from_slice(&encode(&args));

        Ok(CmaVoucher {
            destination: to_checksum(&fields.token, None),
            value: "0x".to_string(),
            payload: format!("0x{}", hex::encode(payload_bytes)),
        })
    } else {
        Err(CmaParserError::IncompatibleInput)
    }
}

pub fn cma_encode_voucher(
    req_type: CmaParserVoucherType,
    app_address: Option<Address>,
    voucher_request: CmaVoucherFieldType,
) -> Result<CmaVoucher, CmaParserError> {
    let result = match req_type {
        CmaParserVoucherType::CmaParserVoucherTypeEther => {
            handle_ether_voucher_encoding(&voucher_request)
        }
        CmaParserVoucherType::CmaParserVoucherTypeErc20 => {
            handle_erc20_voucher_encoding(&voucher_request)
        }
        CmaParserVoucherType::CmaParserVoucherTypeErc721 => {
            let app_address = app_address.ok_or(CmaParserError::MalformedInput)?;
            handle_erc721_voucher_encoding(app_address, &voucher_request)
        }
        CmaParserVoucherType::CmaParserVoucherTypeErc1155Single => {
            let app_address = app_address.ok_or(CmaParserError::MalformedInput)?;
            handle_erc1155_single_voucher_encoding(app_address, &voucher_request)
        }
        CmaParserVoucherType::CmaParserVoucherTypeErc1155Batch => {
            let app_address = app_address.ok_or(CmaParserError::MalformedInput)?;
            handle_erc1155_batch_voucher_encoding(app_address, &voucher_request)
        }
        CmaParserVoucherType::CmaParserVoucherTypeNone => Err(CmaParserError::IncompatibleInput),
    };

    if let Err(ref err) = result {
        set_last_parser_error(err.message());
    }
    result
}
