use crate::types::*;
use ethers::types::{Address, U256, Bytes};
use json::{object, JsonValue};
use hex;

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

    // Bytecode for solidity TransferEther(uint256,bytes32,bytes) = 428c9c4d
    TransferEther = 0x428c9c4d,
    // Bytecode for solidity TransferErc20(address,bytes32,uint256,bytes) = 03d61dcd
    TransferErc20 = 0x03d61dcd,
    // Bytecode for solidity TransferErc721(address,bytes32,uint256,bytes) = af615a5a
    TransferErc721 = 0xaf615a5a,
    // Bytecode for solidity TransferErc1155Single(address,bytes32,uint256,uint256,bytes) = e1c913ed
    TransferErc1155Single = 0xe1c913ed,
    // Bytecode for solidity TransferErc1155Batch(address,bytes32,uint256[],uint256[],bytes) = 638ac6f9
    TransferErc1155Batch = 0x638ac6f9,

    // Bytecode for solidity transfer(address,uint256) = a9059cbb
    Erc20TransferFunctionSelectorFunsel = 0xa9059cbb,
    // Bytecode for solidity safeTransferFrom(address,address,uint256) = 42842e0e
    Erc721TransferFunctionSelectorFunsel = 0x42842e0e,
    // Bytecode for solidity safeTransferFrom(address,address,uint256,uint256,bytes) = f242432a
    Erc1155SingleTransferFunctionSelectorFunsel = 0xf242432a,
    // Bytecode for solidity safeBatchTransferFrom(address,address,uint256[],uint256[],bytes) = 2eb2c2d6
    Erc1155BatchTransferFunctionSelectorFunsel = 0x2eb2c2d6,
}

enum CmaVoucherFieldType {
    EtherVoucherFields(CmaParserEtherVoucherFields),
    Erc20VoucherFields(CmaParserErc20VoucherFields),
    Erc721VoucherFields(CmaParserErc721VoucherFields),
    Erc1155SingleVoucherFields(CmaParserErc1155SingleVoucherFields),
    Erc1155BatchVoucherFields(CmaParserErc1155BatchVoucherFields),
}

enum TokenType {
    Erc20,
    Erc721,
}

enum CmaParserInputType {
    CmaParserInputTypeNone,
    CmaParserInputTypeAuto,
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

enum CmaParserVoucherType {
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
    IncompatibleInput,
    MalformedInput,
    Unknown,
    Message(String),
}

impl CmaParserError {
    pub fn from_code(code: i32) -> Self {
        match code {
            0 => Self::Success,
            -2001 => Self::IncompatibleInput,
            -2002 => Self::MalformedInput,
            -2003 => Self::Unknown,
            -2004 => Self::Message("Unknown error".to_string()),
            _ => Self::Unknown, 
        }
    }
    
    pub fn to_code(self) -> i32 {
        match self {
            Self::Success => 0,
            Self::IncompatibleInput => -2001,
            Self::MalformedInput => -2002,
            Self::Unknown => -2003,
            Self::Message(_) => -2004
        }
    }
}

pub struct CmaVoucher {
    pub address: Address,
    pub value: U256,
    pub data: Bytes,
}

pub struct CmaParserVoucherData {
    pub receiver: Address,
    pub voucher_fields: CmaVoucherFieldType,
}

struct CmaParserEtherVoucherFields {
    pub amount: U256,
}
struct CmaParserErc20VoucherFields {
    pub token: Address,
    pub amount: U256,
}
struct CmaParserErc721VoucherFields {
    pub token: Address,
    pub token_id: U256,
    pub exec_layer_data: Bytes,
}
struct CmaParserErc1155SingleVoucherFields {
    pub token: Address,
    pub token_id: U256,
    pub amount: U256,
}
struct CmaParserErc1155BatchVoucherFields {
    pub token: Address,
    pub count: usize,
    pub token_ids: Vec<U256>,
    pub amounts: Vec<U256>,
}

struct CmaParserEtherDeposit {
    sender: Address,
    amount: U256,
    exec_layer_data: Bytes,
}

struct CmaParserErc20Deposit {
    sender: Address,
    token: Address,
    amount: U256,
    exec_layer_data: Bytes,
}

struct CmaParserErc721Deposit {
    sender: Address,
    token: Address,
    token_id: U256,
    exec_layer_data: Bytes,
}

struct CmaParserErc1155SingleDeposit {
    sender: Address,
    token: Address,
    token_id: U256,
    amount: U256,
    exec_layer_data: Bytes,
}

struct CmaParserErc1155BatchDeposit {
    sender: Address,
    token: Address,
    count: usize,
    token_ids: Vec<U256>,
    amounts: Vec<U256>,
    base_layer_data: Bytes,
    exec_layer_data: Bytes,
}

struct CmaParserEtherWithdrawal {
    receiver: Address,
    amount: U256,
    exec_layer_data: Bytes,
}

struct CmaParserErc20Withdrawal {
    receiver: Address,
    token: Address,
    amount: U256,
    exec_layer_data: Bytes,
}

struct CmaParserErc721Withdrawal {
    receiver: Address,
    token: Address,
    token_id: U256,
    exec_layer_data: Bytes,
}

struct CmaParserErc1155SingleWithdrawal {
    receiver: Address,
    token: Address,
    token_id: U256,
    amount: U256,
    exec_layer_data: Bytes,
}

struct CmaParserErc1155BatchWithdrawal {
    receiver: Address,
    token: Address,
    count: usize,
    token_ids: Vec<U256>,
    amounts: Vec<U256>,
    base_layer_data: Bytes,
    exec_layer_data: Bytes,
}

struct CmaParserEtherTransfer {
    receiver: CmaAccountId,
    amount: U256,
    exec_layer_data: Bytes,
}

struct CmaParserErc20Transfer {
    receiver: CmaAccountId,
    token: Address,
    amount: U256,
    exec_layer_data: Bytes,
}

struct CmaParserErc721Transfer {
    receiver: CmaAccountId,
    token: Address,
    token_id: U256,
    exec_layer_data: Bytes,
}

struct CmaParserErc1155SingleTransfer {
    receiver: CmaAccountId,
    token: Address,
    token_id: U256,
    amount: U256,
    exec_layer_data: Bytes,
}

struct CmaParserErc1155BatchTransfer {
    receiver: CmaAccountId,
    token: Address,
    count: usize,
    token_ids: Vec<U256>,
    amounts: Vec<U256>,
    base_layer_data: Bytes,
    exec_layer_data: Bytes,
}

struct CmaParserBalance {
    account: CmaAccountId,
    token: Address,
    token_id: U256,
    exec_layer_data: Bytes,
}

struct CmaParserSupply {
    token: Address,
    token_id: U256,
    exec_layer_data: Bytes,
}

enum CmaParserInputData {
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
}


pub struct CmaParserInput {
    pub req_type: CmaParserInputType,
    pub input: CmaParserInputData,
}


pub struct CmtRollupAdvance {
    // TODO: Define the structure fields
}

pub struct CmtRollupInspect {
    // TODO: Define the structure fields
}

fn handle_parse_ether_deposit(input: JsonValue) -> Result<CmaParserInputData, CmaParserError> {
    let payload_hex = input["data"]["payload"].as_str().ok_or(CmaParserError::Message(String::from("Invalid payload hex")))?;
    let payload = payload_hex.trim_start_matches("0x");

    let bytes = hex::decode(payload).map_err(|e| CmaParserError::Message(format!("hex decode error: {}", e)))?;

    if bytes.len() < 20 + 32 {
        return Err(CmaParserError::Message("Invalid payload length".to_string()));
    }

    let sender_bytes = &bytes[0..20];
    let sender = Address::from_slice(sender_bytes);

    let value_bytes = &bytes[20..52];
    let value = U256::from_big_endian(value_bytes);

    let exec_layer_data = Bytes::from(bytes[52..].to_vec());

    Ok(CmaParserInputData::EtherDeposit(CmaParserEtherDeposit {
        sender,
        amount: value,
        exec_layer_data,
    }))
}

fn handle_parse_erc20_and_erc721_deposit(input: JsonValue, t_type: TokenType) -> Result<CmaParserInputData, CmaParserError> {
    let payload_hex = input["data"]["payload"].as_str().ok_or(CmaParserError::Message(String::from("Invalid payload hex")))?;
    let payload = payload_hex.trim_start_matches("0x");

    let bytes = hex::decode(payload).map_err(|e| CmaParserError::Message(format!("hex decode error: {}", e)))?;

    if bytes.len() < 20 + 20 + 32 {
        return Err(CmaParserError::Message("Invalid payload length".to_string()));
    }

    let token = &bytes[0..20];
    let sender = &bytes[20..40];
    let amount_bytes = &bytes[40..72];
    let amount = U256::from_big_endian(amount_bytes);
    let exec_layer_data = Bytes::from(bytes[72..].to_vec());

    match t_type {
        TokenType::Erc20 => Ok(CmaParserInputData::Erc20Deposit(CmaParserErc20Deposit {
            sender: Address::from_slice(sender),
            token: Address::from_slice(token),
            amount,
            exec_layer_data,
        })),
        TokenType::Erc721 => Ok(CmaParserInputData::Erc721Deposit(CmaParserErc721Deposit {
            sender: Address::from_slice(sender),
            token: Address::from_slice(token),
            token_id: amount,
            exec_layer_data,
        }))
    }
}

fn handle_parse_erc1155_single_deposit(input: JsonValue) -> Result<CmaParserInputData, CmaParserError> {
    let payload_hex = input["data"]["payload"].as_str().ok_or(CmaParserError::Message(String::from("Invalid payload hex")))?;
    let payload = payload_hex.trim_start_matches("0x");

    let bytes = hex::decode(payload).map_err(|e| CmaParserError::Message(format!("hex decode error: {}", e)))?;

    if bytes.len() < 20 + 20 + 32 + 32 {
        return Err(CmaParserError::Message("Invalid payload length".to_string()));
    }

    let token = &bytes[0..20];
    let sender = &bytes[20..40];
    let token_id_bytes = &bytes[40..72];
    let token_id = U256::from_big_endian(token_id_bytes);
    let amount_bytes = &bytes[72..104];
    let amount = U256::from_big_endian(amount_bytes);
    let _base_layer_data = Bytes::from(bytes[104..136].to_vec());
    let exec_layer_data = Bytes::from(bytes[136..].to_vec());

    Ok(CmaParserInputData::Erc1155SingleDeposit(CmaParserErc1155SingleDeposit {
        sender: Address::from_slice(sender),
        token: Address::from_slice(token),
        token_id,
        amount,
        exec_layer_data,
    }) )
}


fn handle_parse_erc1155_batch_deposit(input: JsonValue) -> Result<CmaParserInputData, CmaParserError> {
    let payload_hex = input["data"]["payload"].as_str().ok_or(CmaParserError::Message(String::from("Invalid payload hex")))?;
    let payload = payload_hex.trim_start_matches("0x");

    let bytes = hex::decode(payload).map_err(|e| CmaParserError::Message(format!("hex decode error: {}", e)))?;

    if bytes.len() < 20 + 20 + 32 + 32 + 32 {
        return Err(CmaParserError::Message("Invalid payload length".to_string()));
    }

    let u256_from = |b: &[u8]| U256::from_big_endian(b);
    let as_addr = |b: &[u8]| Address::from_slice(&b[12..32]);

    let token   = as_addr(&bytes[0..32]);
    let sender  = as_addr(&bytes[32..64]);

    let token_ids_offset = u256_from(&bytes[64..96]).as_usize();
    let values_offset    = u256_from(&bytes[96..128]).as_usize();
    let base_offset      = u256_from(&bytes[128..160]).as_usize();
    let exec_offset      = u256_from(&bytes[160..192]).as_usize();

    let token_ids_len = u256_from(&bytes[token_ids_offset..token_ids_offset+32]).as_usize();
    let mut token_ids = Vec::with_capacity(token_ids_len);

    let mut cursor = token_ids_offset + 32;
    for _ in 0..token_ids_len {
        token_ids.push(u256_from(&bytes[cursor..cursor+32]));
        cursor += 32;
    }

    let values_len = u256_from(&bytes[values_offset..values_offset+32]).as_usize();
    let mut values = Vec::with_capacity(values_len);

    let mut cursor2 = values_offset + 32;
    for _ in 0..values_len {
        values.push(u256_from(&bytes[cursor2..cursor2+32]));
        cursor2 += 32;
    }

    let base_len = u256_from(&bytes[base_offset..base_offset+32]).as_usize();
    let base_start = base_offset + 32;
    let base_end = base_start + base_len;
    let base_layer_data = Bytes::from(bytes[base_start..base_end].to_vec());

    let exec_len = u256_from(&bytes[exec_offset..exec_offset+32]).as_usize();
    let exec_start = exec_offset + 32;
    let exec_end = exec_start + exec_len;
    let exec_layer_data = Bytes::from(bytes[exec_start..exec_end].to_vec());

    if token_ids_len == 0 || values_len == 0 || token_ids_len != values_len {
        return Err(CmaParserError::Message("Invalid payload data".to_string()));
    }

    Ok(CmaParserInputData::Erc1155BatchDeposit(CmaParserErc1155BatchDeposit {
        sender,
        token,
        count: token_ids_len,
        token_ids,
        amounts: values,
        base_layer_data,
        exec_layer_data,
    }))
}

pub fn cma_decode_advance(req_type:  CmaParserInputType, input: JsonValue) -> Result<CmaParserInput, CmaParserError> {

    match req_type {
        CmaParserInputType::CmaParserInputTypeNone => {

            Err(CmaParserError::Unknown) // Placeholder
        },
        CmaParserInputType::CmaParserInputTypeAuto => {

            Err(CmaParserError::Unknown) // Placeholder
        },
        CmaParserInputType::CmaParserInputTypeEtherDeposit => {
            return handle_parse_ether_deposit(input).map(|data| CmaParserInput {
                    req_type,
                    input: data,
                });
            },
        CmaParserInputType::CmaParserInputTypeErc20Deposit => {
            return handle_parse_erc20_and_erc721_deposit(input, TokenType::Erc20).map(|data| CmaParserInput {
                    req_type,
                    input: data,
                });
        },
        CmaParserInputType::CmaParserInputTypeErc721Deposit => {
            return handle_parse_erc20_and_erc721_deposit(input, TokenType::Erc721).map(|data| CmaParserInput {
                    req_type,
                    input: data,
                });
        },
        CmaParserInputType::CmaParserInputTypeErc1155SingleDeposit => {
            return handle_parse_erc1155_single_deposit(input).map(|data| CmaParserInput {
                    req_type,
                    input: data,
                });
        },
        CmaParserInputType::CmaParserInputTypeErc1155BatchDeposit => {
            return handle_parse_erc1155_batch_deposit(input).map(|data| CmaParserInput {
                    req_type,
                    input: data,
                });
        },
        CmaParserInputType::CmaParserInputTypeEtherWithdrawal => {

            // Return CmaParserInput with Erc20Deposit data
            Err(CmaParserError::Unknown) // Placeholder
        },

        CmaParserInputType::CmaParserInputTypeErc20Withdrawal => {

            // Return CmaParserInput with EtherDeposit data
            Err(CmaParserError::Unknown) // Placeholder
        },
        CmaParserInputType::CmaParserInputTypeErc721Withdrawal => {

            // Return CmaParserInput with Erc20Deposit data
            Err(CmaParserError::Unknown) // Placeholder
        },
        CmaParserInputType::CmaParserInputTypeErc1155SingleWithdrawal => {

            // Return CmaParserInput with EtherDeposit data
            Err(CmaParserError::Unknown) // Placeholder
        },
        CmaParserInputType::CmaParserInputTypeErc1155BatchWithdrawal => {

            // Return CmaParserInput with Erc20Deposit data
            Err(CmaParserError::Unknown) // Placeholder
        },
        CmaParserInputType::CmaParserInputTypeEtherTransfer => {

            // Return CmaParserInput with EtherDeposit data
            Err(CmaParserError::Unknown) // Placeholder
        },
        CmaParserInputType::CmaParserInputTypeErc20Transfer => {

            // Return CmaParserInput with Erc20Deposit data
            Err(CmaParserError::Unknown) // Placeholder
        },
        CmaParserInputType::CmaParserInputTypeErc721Transfer => {

            // Return CmaParserInput with EtherDeposit data
            Err(CmaParserError::Unknown) // Placeholder
        },
        CmaParserInputType::CmaParserInputTypeErc1155SingleTransfer => {

            // Return CmaParserInput with Erc20Deposit data
            Err(CmaParserError::Unknown) // Placeholder
        },
        CmaParserInputType::CmaParserInputTypeErc1155BatchTransfer => {

            // Return CmaParserInput with Erc20Deposit data
            Err(CmaParserError::Unknown) // Placeholder
        },
        CmaParserInputType::CmaParserInputTypeBalance => {

            // Return CmaParserInput with EtherDeposit data
            Err(CmaParserError::Unknown) // Placeholder
        },
        CmaParserInputType::CmaParserInputTypeSupply => {

            // Return CmaParserInput with Erc20Deposit data
            Err(CmaParserError::Unknown) // Placeholder
        },
        // Handle other request types similarly...
        _ => Err(CmaParserError::IncompatibleInput),
    }
}

pub fn cma_decode_inspect(req_type:  CmaParserInputType, input: CmtRollupInspect) -> Result<CmaParserInput, CmaParserError> {

}

pub fn cma_encode_voucher (req_type: CmaParserVoucherType, app_address: Address, voucher_request: CmaParserVoucherData ) -> Result<CmaVoucher, CmaParserError> {

}

