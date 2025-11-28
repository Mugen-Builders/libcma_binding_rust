use crate::types::*;
use ethers::types::{Address, U256, Bytes};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CmaParserError {
    Success,
    IncompatibleInput,
    MalformedInput,
    Unknown,
}

impl CmaParserError {
    pub fn from_code(code: i32) -> Self {
        match code {
            0 => Self::Success,
            -2001 => Self::IncompatibleInput,
            -2002 => Self::MalformedInput,
            -2003 => Self::Unknown,
            _ => Self::Unknown, 
        }
    }
    
    pub fn to_code(self) -> i32 {
        match self {
            Self::Success => 0,
            Self::IncompatibleInput => -2001,
            Self::MalformedInput => -2002,
            Self::Unknown => -2003,
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



pub fn cma_decode_advance(req_type:  CmaParserInputType, input: CmtRollupAdvance) -> Result<CmaParserInput, CmaParserError> {

}

pub fn cma_decode_inspect(req_type:  CmaParserInputType, input: CmtRollupInspect) -> Result<CmaParserInput, CmaParserError> {

}

pub fn cma_encode_voucher (req_type: CmaParserVoucherType, app_address: Address, voucher_request: CmaParserVoucherData ) -> Result<CmaVoucher, CmaParserError> {

}

