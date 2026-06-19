use crate::bindings;
// use std::fmt;
pub use ethers_core::types::{Address, U256};
use std::str::FromStr;

pub const ADDRESS_LENGTH: usize = 20;
pub const U256_LENGTH: usize = 32;

pub trait AddressCBindingsExt {
    fn new(bytes: [u8; ADDRESS_LENGTH]) -> Self;
    fn from_str_hex(s: &str) -> Result<Self, String>
    where
        Self: Sized;
    fn from_slice(slice: &[u8]) -> Result<Self, String>
    where
        Self: Sized;
    fn as_array(&self) -> [u8; ADDRESS_LENGTH];
    fn as_bytes(&self) -> &[u8; ADDRESS_LENGTH];
    fn to_c(&self) -> bindings::cmt_abi_address_t;
    fn from_c(c_addr: &bindings::cmt_abi_address_t) -> Self;
}

impl AddressCBindingsExt for Address {
    fn new(bytes: [u8; ADDRESS_LENGTH]) -> Self {
        Address::from(bytes)
    }

    fn from_str_hex(s: &str) -> Result<Self, String> {
        Address::from_str(s).map_err(|e| e.to_string())
    }

    fn from_slice(slice: &[u8]) -> Result<Self, String> {
        if slice.len() != ADDRESS_LENGTH {
            return Err(format!(
                "Address must be {} bytes, got {}",
                ADDRESS_LENGTH,
                slice.len()
            ));
        }
        Ok(Address::from_slice(slice))
    }

    fn as_array(&self) -> [u8; ADDRESS_LENGTH] {
        self.0
    }

    fn as_bytes(&self) -> &[u8; ADDRESS_LENGTH] {
        &self.0
    }

    fn to_c(&self) -> bindings::cmt_abi_address_t {
        bindings::cmt_abi_address_t { data: self.as_array() }
    }

    fn from_c(c_addr: &bindings::cmt_abi_address_t) -> Self {
        Address::from(c_addr.data)
    }
}

pub trait U256CBindingsExt {
    fn new(bytes: [u8; U256_LENGTH]) -> Self
    where
        Self: Sized;
    fn zero() -> Self
    where
        Self: Sized;
    fn from_u64(value: u64) -> Self
    where
        Self: Sized;
    fn from_be_bytes(bytes: [u8; U256_LENGTH]) -> Self
    where
        Self: Sized;
    fn from_slice(slice: &[u8]) -> Result<Self, String>
    where
        Self: Sized;
    fn as_be_bytes(&self) -> [u8; U256_LENGTH];
    fn to_u128_opt(&self) -> Option<u128>;
    fn to_c(&self) -> bindings::cmt_abi_u256_t;
    fn from_c(c_u256: &bindings::cmt_abi_u256_t) -> Self;
}

impl U256CBindingsExt for U256 {
    fn new(bytes: [u8; U256_LENGTH]) -> Self {
        U256::from_big_endian(&bytes)
    }

    fn zero() -> Self {
        U256::zero()
    }

    fn from_u64(value: u64) -> Self {
        U256::from(value)
    }

    fn from_be_bytes(bytes: [u8; U256_LENGTH]) -> Self {
        U256::from_big_endian(&bytes)
    }

    fn from_slice(slice: &[u8]) -> Result<Self, String> {
        if slice.len() != U256_LENGTH {
            return Err(format!("U256 must be 32 bytes, got {}", slice.len()));
        }
        Ok(U256::from_big_endian(slice))
    }

    fn as_be_bytes(&self) -> [u8; U256_LENGTH] {
        let mut out = [0u8; U256_LENGTH];
        self.to_big_endian(&mut out);
        out
    }

    fn to_u128_opt(&self) -> Option<u128> {
        if self.bits() > 128 {
            None
        } else {
            Some(self.low_u128())
        }
    }

    fn to_c(&self) -> bindings::cmt_abi_u256_t {
        bindings::cmt_abi_u256_t { data: self.as_be_bytes() }
    }

    fn from_c(c_u256: &bindings::cmt_abi_u256_t) -> Self {
        U256::from_big_endian(&c_u256.data)
    }
}

/// Account ID (32 bytes)
pub type AccountId = U256;

/// Token ID (32 bytes)
pub type TokenId = U256;

/// Token address (20 bytes)
pub type TokenAddress = Address;

/// Ledger account ID (64-bit)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LedgerAccountId(pub u64);

/// Ledger asset ID (64-bit)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LedgerAssetId(pub u64);

/// Ledger retrieve operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetrieveOperation {
    Find,
    Create,
    FindOrCreate,
    FindAndRemove,
}

impl RetrieveOperation {
    pub fn to_c(&self) -> bindings::cma_ledger_retrieve_operation_t {
        match self {
            RetrieveOperation::Find => bindings::cma_ledger_retrieve_operation_t_CMA_LEDGER_OP_FIND,
            RetrieveOperation::Create => {
                bindings::cma_ledger_retrieve_operation_t_CMA_LEDGER_OP_CREATE
            }
            RetrieveOperation::FindOrCreate => {
                bindings::cma_ledger_retrieve_operation_t_CMA_LEDGER_OP_FIND_OR_CREATE
            }
            RetrieveOperation::FindAndRemove => {
                bindings::cma_ledger_retrieve_operation_t_CMA_LEDGER_OP_FIND_AND_REMOVE
            }
        }
    }
}

/// Asset type for ledger operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetType {
    Id,
    Base,
    TokenAddress,
    TokenAddressId,
    TokenAddressIdAmount,
}

impl AssetType {
    pub fn to_c(&self) -> bindings::cma_ledger_asset_type_t {
        match self {
            AssetType::Id => bindings::cma_ledger_asset_type_t_CMA_LEDGER_ASSET_TYPE_ID,
            AssetType::Base => bindings::cma_ledger_asset_type_t_CMA_LEDGER_ASSET_TYPE_BASE,
            AssetType::TokenAddress => {
                bindings::cma_ledger_asset_type_t_CMA_LEDGER_ASSET_TYPE_TOKEN_ADDRESS
            }
            AssetType::TokenAddressId => {
                bindings::cma_ledger_asset_type_t_CMA_LEDGER_ASSET_TYPE_TOKEN_ADDRESS_ID
            }
            AssetType::TokenAddressIdAmount => {
                bindings::cma_ledger_asset_type_t_CMA_LEDGER_ASSET_TYPE_TOKEN_ADDRESS_ID_AMOUNT
            }
        }
    }
}

/// Account type for ledger operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountType {
    Id,
    WalletAddress,
    AccountId,
}

impl AccountType {
    pub fn to_c(&self) -> bindings::cma_ledger_account_type_t {
        match self {
            AccountType::Id => bindings::cma_ledger_account_type_t_CMA_LEDGER_ACCOUNT_TYPE_ID,
            AccountType::WalletAddress => {
                bindings::cma_ledger_account_type_t_CMA_LEDGER_ACCOUNT_TYPE_WALLET_ADDRESS
            }
            AccountType::AccountId => {
                bindings::cma_ledger_account_type_t_CMA_LEDGER_ACCOUNT_TYPE_ACCOUNT_ID
            }
        }
    }
}

/// Owned byte buffer that can be safely passed to C as `cmt_abi_bytes_t`.
#[derive(Debug, Clone)]
pub struct OwnedCBytes {
    inner: Vec<u8>,
}

impl OwnedCBytes {
    pub fn new(data: impl Into<Vec<u8>>) -> Self {
        Self {
            inner: data.into(),
        }
    }

    pub fn as_c_bytes(&self) -> bindings::cmt_abi_bytes_t {
        bindings::cmt_abi_bytes_t {
            data: self.inner.as_ptr() as *mut std::ffi::c_void,
            length: self.inner.len(),
        }
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.inner
    }
}

/// Helper to convert bytes into an owned C-compatible buffer wrapper.
pub fn bytes_to_c_bytes(data: &[u8]) -> OwnedCBytes {
    OwnedCBytes::new(data.to_vec())
}

/// Helper to safely extract bytes from C bytes struct
pub unsafe fn c_bytes_to_vec(c_bytes: &bindings::cmt_abi_bytes_t) -> Vec<u8> {
    if c_bytes.data.is_null() || c_bytes.length == 0 {
        return Vec::new();
    }
    std::slice::from_raw_parts(c_bytes.data as *const u8, c_bytes.length).to_vec()
}

pub type CmaAccountId = String;
