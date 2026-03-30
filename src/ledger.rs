use crate::bindings;
use crate::error::LedgerError;
use crate::types::*;
use std::ffi::CString;
use std::path::Path;
use std::ptr;

/// Storage mode for file-backed ledgers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LedgerMemoryMode {
    OpenOnly,
    CreateOnly,
}

impl LedgerMemoryMode {
    fn to_c(self) -> bindings::cma_ledger_memory_mode_t {
        match self {
            LedgerMemoryMode::OpenOnly => bindings::cma_ledger_memory_mode_t_CMA_LEDGER_OPEN_ONLY,
            LedgerMemoryMode::CreateOnly => {
                bindings::cma_ledger_memory_mode_t_CMA_LEDGER_CREATE_ONLY
            }
        }
    }
}

/// Configuration for file-backed ledger initialization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LedgerFileConfig {
    pub mode: LedgerMemoryMode,
    pub offset: usize,
    pub memory_length: usize,
    pub max_accounts: usize,
    pub max_assets: usize,
    pub max_balances: usize,
}

impl Default for LedgerFileConfig {
    fn default() -> Self {
        Self {
            mode: LedgerMemoryMode::CreateOnly,
            offset: 0,
            memory_length: 1024 * 1024,
            max_accounts: 256,
            max_assets: 256,
            max_balances: 1024,
        }
    }
}

/// Configuration for buffer-backed ledger initialization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LedgerBufferConfig {
    pub max_accounts: usize,
    pub max_assets: usize,
    pub max_balances: usize,
}

impl Default for LedgerBufferConfig {
    fn default() -> Self {
        Self {
            max_accounts: 256,
            max_assets: 256,
            max_balances: 1024,
        }
    }
}

/// Safe wrapper around the C ledger
pub struct Ledger {
    inner: bindings::cma_ledger_t,
}

impl Ledger {
    fn restore_empty_ledger(&mut self) {
        unsafe {
            let _ = bindings::cma_ledger_init(&mut self.inner);
        }
    }

    fn reinitialize(
        &mut self,
        init_fn: impl FnOnce(*mut bindings::cma_ledger_t) -> i32,
    ) -> Result<(), LedgerError> {
        unsafe {
            let fini_result = bindings::cma_ledger_fini(&mut self.inner);
            if fini_result < 0 {
                return Err(LedgerError::from_code(fini_result));
            }

            let init_result = init_fn(&mut self.inner);
            if init_result < 0 {
                self.restore_empty_ledger();
                return Err(LedgerError::from_code(init_result));
            }

            Ok(())
        }
    }

    /// Initialize a new ledger
    pub fn new() -> Result<Self, LedgerError> {
        unsafe {
            let mut ledger = std::mem::zeroed::<bindings::cma_ledger_t>();
            let result = bindings::cma_ledger_init(&mut ledger);
            if result < 0 {
                return Err(LedgerError::from_code(result));
            }
            Ok(Ledger { inner: ledger })
        }
    }

    /// Reset the ledger
    pub fn reset(&mut self) -> Result<(), LedgerError> {
        unsafe {
            let result = bindings::cma_ledger_reset(&mut self.inner);
            if result < 0 {
                return Err(LedgerError::from_code(result));
            }
            Ok(())
        }
    }

    /// Retrieve or create an asset
    pub fn retrieve_asset(
        &mut self,
        asset_id: Option<LedgerAssetId>,
        token_address: Option<TokenAddress>,
        token_id: Option<TokenId>,
        asset_type: AssetType,
        operation: RetrieveOperation,
    ) -> Result<LedgerAssetId, LedgerError> {
        unsafe {
            let mut out_asset_id = asset_id.map(|id| id.0).unwrap_or(0);
            let mut c_asset_type = asset_type.to_c();
            let mut out_token_address = token_address
                .map(|addr| addr.to_c())
                .unwrap_or_else(|| bindings::cmt_abi_address_t { data: [0u8; 20] });
            let mut out_token_id = token_id
                .map(|id| id.to_c())
                .unwrap_or_else(|| bindings::cmt_abi_u256_t { data: [0u8; 32] });

            let result = bindings::cma_ledger_retrieve_asset(
                &mut self.inner,
                &mut out_asset_id,
                if token_address.is_some() {
                    &mut out_token_address
                } else {
                    ptr::null_mut()
                },
                if token_id.is_some() {
                    &mut out_token_id
                } else {
                    ptr::null_mut()
                },
                &mut c_asset_type,
                operation.to_c(),
            );

            if result < 0 {
                return Err(LedgerError::from_code(result));
            }

            Ok(LedgerAssetId(out_asset_id))
        }
    }

    pub fn retrieve_erc20_asset_via_address(
        &mut self,
        token_address: Address,
    ) -> Result<LedgerAssetId, LedgerError> {
        let asset_id = self
            .retrieve_asset(
                None,
                Some(token_address),
                None,
                AssetType::TokenAddress,
                RetrieveOperation::FindOrCreate,
            )
            .map_err(|e| e)?;

        return Ok(asset_id);
    }

    pub fn retrieve_erc721_assets_via_address(
        &mut self,
        token_address: Address,
        token_id: U256,
    ) -> Result<LedgerAssetId, LedgerError> {
        let asset_id = self
            .retrieve_asset(
                None,
                Some(token_address),
                Some(token_id),
                AssetType::TokenAddressId,
                RetrieveOperation::FindOrCreate,
            )
            .map_err(|e| e)?;

        return Ok(asset_id);
    }

    pub fn retrieve_ether_assets(&mut self) -> Result<LedgerAssetId, LedgerError> {
        let asset_id = self
            .retrieve_asset(
                None,
                None,
                None,
                AssetType::TokenAddressId,
                RetrieveOperation::FindOrCreate,
            )
            .map_err(|e| e)?;

        return Ok(asset_id);
    }

    /// Retrieve or create an account
    pub fn retrieve_account(
        &mut self,
        account_id: Option<LedgerAccountId>,
        account_type: AccountType,
        operation: RetrieveOperation,
        addr_or_id: Option<&[u8]>,
    ) -> Result<LedgerAccountId, LedgerError> {
        unsafe {
            let mut out_account_id = account_id.map(|id| id.0).unwrap_or(0);
            let mut c_account_type = account_type.to_c();
            let mut c_account = std::mem::zeroed::<bindings::cma_ledger_account_t>();
            c_account.type_ = c_account_type;

            let addr_ptr = if let Some(addr) = addr_or_id {
                addr.as_ptr() as *const std::ffi::c_void
            } else {
                ptr::null()
            };

            let result = bindings::cma_ledger_retrieve_account(
                &mut self.inner,
                &mut out_account_id,
                &mut c_account,
                addr_ptr,
                &mut c_account_type,
                operation.to_c(),
            );

            if result < 0 {
                return Err(LedgerError::from_code(result));
            }

            Ok(LedgerAccountId(out_account_id))
        }
    }

    /// Simple Retrieve or create account
    pub fn retrieve_account_via_address(
        &mut self,
        account_address: Address,
    ) -> Result<LedgerAccountId, LedgerError> {
        let account_id = self
            .retrieve_account(
                None,
                AccountType::WalletAddress,
                RetrieveOperation::FindOrCreate,
                Some(account_address.as_bytes()),
            )
            .map_err(|e| e)?;

        return Ok(account_id);
    }

    /// Deposit assets to an account
    pub fn deposit(
        &mut self,
        asset_id: LedgerAssetId,
        to_account_id: LedgerAccountId,
        amount: U256,
    ) -> Result<(), LedgerError> {
        unsafe {
            let c_amount = amount.to_c();
            let result = bindings::cma_ledger_deposit(
                &mut self.inner,
                asset_id.0,
                to_account_id.0,
                &c_amount,
            );

            if result < 0 {
                return Err(LedgerError::from_code(result));
            }

            Ok(())
        }
    }

    /// Withdraw assets from an account
    pub fn withdraw(
        &mut self,
        asset_id: LedgerAssetId,
        from_account_id: LedgerAccountId,
        amount: U256,
    ) -> Result<(), LedgerError> {
        unsafe {
            let c_amount = amount.to_c();
            let result = bindings::cma_ledger_withdraw(
                &mut self.inner,
                asset_id.0,
                from_account_id.0,
                &c_amount,
            );

            if result < 0 {
                return Err(LedgerError::from_code(result));
            }

            Ok(())
        }
    }

    /// Transfer assets between accounts
    pub fn transfer(
        &mut self,
        asset_id: LedgerAssetId,
        from_account_id: LedgerAccountId,
        to_account_id: LedgerAccountId,
        amount: U256,
    ) -> Result<(), LedgerError> {
        unsafe {
            let c_amount = amount.to_c();
            let result = bindings::cma_ledger_transfer(
                &mut self.inner,
                asset_id.0,
                from_account_id.0,
                to_account_id.0,
                &c_amount,
            );

            if result < 0 {
                return Err(LedgerError::from_code(result));
            }

            Ok(())
        }
    }

    /// Get balance for an account
    pub fn get_balance(
        &self,
        asset_id: LedgerAssetId,
        account_id: LedgerAccountId,
    ) -> Result<U256, LedgerError> {
        unsafe {
            let mut out_balance = std::mem::zeroed::<bindings::cmt_abi_u256_t>();
            let result = bindings::cma_ledger_get_balance(
                &self.inner as *const _ as *mut _,
                asset_id.0,
                account_id.0,
                &mut out_balance,
            );

            if result < 0 {
                return Err(LedgerError::from_code(result));
            }

            Ok(U256::from_c(&out_balance))
        }
    }

    /// Get total supply for an asset
    pub fn get_total_supply(&self, asset_id: LedgerAssetId) -> Result<U256, LedgerError> {
        unsafe {
            let mut out_supply = std::mem::zeroed::<bindings::cmt_abi_u256_t>();
            let result = bindings::cma_ledger_get_total_supply(
                &self.inner as *const _ as *mut _,
                asset_id.0,
                &mut out_supply,
            );

            if result < 0 {
                return Err(LedgerError::from_code(result));
            }

            Ok(U256::from_c(&out_supply))
        }
    }

    // get last error message
    pub fn get_last_error_message(&self) -> Result<String, LedgerError> {
        unsafe {
            let msg = bindings::cma_ledger_get_last_error_message();
            if msg.is_null() {
                return Err(LedgerError::Unknown);
            }
            Ok(std::ffi::CStr::from_ptr(msg).to_string_lossy().to_string())
        }
    }

    /// Reinitialize this ledger using file-backed storage.
    pub fn init_from_file<P: AsRef<Path>>(
        &mut self,
        file_path: P,
        config: LedgerFileConfig,
    ) -> Result<(), LedgerError> {
        let file_path = CString::new(file_path.as_ref().to_string_lossy().as_bytes())
            .map_err(|_| LedgerError::Other(-22))?;

        self.reinitialize(|ledger| unsafe {
            bindings::cma_ledger_init_file(
                ledger,
                file_path.as_ptr(),
                config.mode.to_c(),
                config.offset,
                config.memory_length,
                config.max_accounts,
                config.max_assets,
                config.max_balances,
            )
        })
    }

    /// Reinitialize this ledger using caller-provided memory.
    pub fn init_from_buffer(
        &mut self,
        buffer: &mut [u8],
        config: LedgerBufferConfig,
    ) -> Result<(), LedgerError> {
        self.reinitialize(|ledger| unsafe {
            bindings::cma_ledger_init_buffer(
                ledger,
                buffer.as_mut_ptr() as *mut _,
                buffer.len(),
                config.max_accounts,
                config.max_assets,
                config.max_balances,
            )
        })
    }
}

impl Drop for Ledger {
    fn drop(&mut self) {
        unsafe {
            bindings::cma_ledger_fini(&mut self.inner);
        }
    }
}

impl Default for Ledger {
    fn default() -> Self {
        Self::new().expect("Failed to initialize ledger")
    }
}
