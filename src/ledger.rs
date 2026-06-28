use crate::bindings;
use crate::error::LedgerError;
use crate::types::*;
use std::ffi::CString;
use std::path::Path;
use std::ptr;

/// Configuration for file-backed ledger initialization.
///
/// The backing file is now always opened in create-or-open mode: it is created
/// when missing and validated (size/version) when it already exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LedgerFileConfig {
    pub offset: usize,
    pub memory_length: usize,
    pub max_accounts: usize,
    pub max_assets: usize,
    pub max_balances: usize,
}

impl Default for LedgerFileConfig {
    fn default() -> Self {
        Self {
            offset: 0,
            memory_length: 1024 * 1024,
            max_accounts: 256,
            max_assets: 256,
            max_balances: 1024,
        }
    }
}

/// The single, immutable asset that a single-asset ledger tracks.
///
/// Chosen once when the ledger is created and fixed for the lifetime of the
/// backing store. Reopening the same file with a different asset is rejected by
/// the underlying library.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LedgerAsset {
    /// The base asset (ether). No token address.
    Ether,
    /// A single ERC-20 token, identified by its contract address.
    Erc20(TokenAddress),
}

impl LedgerAsset {
    /// Lower to the C `(asset_type, token_address)` pair. The address is returned
    /// by value so the caller can keep it alive while passing a pointer to it.
    fn to_c(self) -> (bindings::cma_ledger_asset_type_t, Option<bindings::cma_token_address_t>) {
        match self {
            LedgerAsset::Ether => (
                bindings::cma_ledger_asset_type_t_CMA_LEDGER_ASSET_TYPE_BASE,
                None,
            ),
            LedgerAsset::Erc20(addr) => (
                bindings::cma_ledger_asset_type_t_CMA_LEDGER_ASSET_TYPE_TOKEN_ADDRESS,
                Some(addr.to_c()),
            ),
        }
    }
}

/// Configuration for file-backed single-asset ledger initialization.
///
/// Unlike [`LedgerFileConfig`], a single-asset ledger has no `max_assets`
/// (there is exactly one) or `max_balances`; `max_accounts` is the capacity of
/// the withdrawable-balance drive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LedgerSingleFileConfig {
    pub offset: usize,
    pub memory_length: usize,
    pub max_accounts: usize,
}

impl Default for LedgerSingleFileConfig {
    fn default() -> Self {
        Self {
            offset: 0,
            memory_length: 1024 * 1024,
            max_accounts: 256,
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
                ptr::null_mut(),
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
        self.retrieve_asset(
            None,
            None,
            None,
            AssetType::Base,
            RetrieveOperation::FindOrCreate,
        )
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
                ptr::null_mut(),
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
                ptr::null_mut(),
            );

            if result < 0 {
                return Err(LedgerError::from_code(result));
            }

            Ok(U256::from_c(&out_balance))
        }
    }

    /// Get total supply for an asset (via [`cma_ledger_retrieve_asset`] with find).
    pub fn get_total_supply(&self, asset_id: LedgerAssetId) -> Result<U256, LedgerError> {
        unsafe {
            let mut asset_id_mut = asset_id.0;
            let mut asset_type = bindings::cma_ledger_asset_type_t_CMA_LEDGER_ASSET_TYPE_ID;
            let mut out_supply = std::mem::zeroed::<bindings::cma_amount_t>();
            let result = bindings::cma_ledger_retrieve_asset(
                &self.inner as *const _ as *mut _,
                &mut asset_id_mut,
                ptr::null_mut(),
                ptr::null_mut(),
                &mut out_supply,
                &mut asset_type,
                bindings::cma_ledger_retrieve_operation_t_CMA_LEDGER_OP_FIND,
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

    /// Reinitialize this ledger as a single-asset ledger backed by a file
    /// (create-or-open). The asset (ether or one ERC-20) is fixed for the life
    /// of the backing file; balances are 64-bit.
    pub fn init_single_from_file<P: AsRef<Path>>(
        &mut self,
        file_path: P,
        config: LedgerSingleFileConfig,
        asset: LedgerAsset,
    ) -> Result<(), LedgerError> {
        let file_path = CString::new(file_path.as_ref().to_string_lossy().as_bytes())
            .map_err(|_| LedgerError::Other(-22))?;
        let (asset_type, token_address) = asset.to_c();

        self.reinitialize(|ledger| unsafe {
            bindings::cma_ledger_init_single_file(
                ledger,
                file_path.as_ptr(),
                config.offset,
                config.memory_length,
                config.max_accounts,
                asset_type,
                token_address
                    .as_ref()
                    .map(|addr| addr as *const _)
                    .unwrap_or(ptr::null()),
            )
        })
    }

    /// Reinitialize this ledger as a single-asset ledger over caller-provided
    /// memory (non-persistent). `max_accounts` is the capacity of the
    /// withdrawable-balance drive.
    pub fn init_single_from_buffer(
        &mut self,
        buffer: &mut [u8],
        max_accounts: usize,
        asset: LedgerAsset,
    ) -> Result<(), LedgerError> {
        let (asset_type, token_address) = asset.to_c();

        self.reinitialize(|ledger| unsafe {
            bindings::cma_ledger_init_single_buffer(
                ledger,
                buffer.as_mut_ptr() as *mut _,
                buffer.len(),
                max_accounts,
                asset_type,
                token_address
                    .as_ref()
                    .map(|addr| addr as *const _)
                    .unwrap_or(ptr::null()),
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
