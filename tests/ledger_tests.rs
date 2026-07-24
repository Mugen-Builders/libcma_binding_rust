//! Behavioural tests for the libcma `Ledger`.
//!
//! ## Asset types — fungible vs. non-fungible
//!
//! libcma has two token-backed asset types, and the distinction is load-bearing:
//!
//! * [`AssetType::TokenAddress`] — a **fungible** token (ERC-20), keyed by token address
//!   only. Its total supply is a full 256-bit integer, so any balance is valid.
//! * [`AssetType::TokenAddressId`] — a **non-fungible** token (ERC-721 / a single ERC-1155
//!   id), keyed by *(token address, token id)*. Such an asset is unique: real libcma
//!   enforces that its supply can only ever go `0 -> 1` and that the only legal deposit is
//!   exactly `1` (`src/ledger_impl.cpp`). Depositing e.g. `1000` returns `SupplyOverflow` —
//!   that error is the "you violated NFT uniqueness" signal, **not** an arithmetic overflow
//!   (the supply field is 256-bit and nowhere near full).
//!
//! These fungible tests therefore use `TokenAddress`. The mock backend does not enforce the
//! NFT rule, so an earlier version of these tests used `TokenAddressId` with fungible amounts
//! and only passed against the mock; they failed against real libcma. See
//! `test_nft_asset_deposit_is_capped_at_one` for the enforced NFT behaviour.
//!
//! ## Backend-agnostic assertions
//!
//! These run under BOTH the default `mock` backend and the real libcma backends
//! (`host-real` / `riscv64`), which differ in incidental ways — most notably the mock uses
//! 1-based asset/account ids (reserving id 1 for the Base asset) while real libcma is 0-based.
//! So the tests assert *relationships* (a freshly created id is found again unchanged) rather
//! than absolute id values, and probe "not found" via the `Find` retrieve operation (which
//! errors on both) rather than `get_balance` (which returns 0 for an unknown pair on real
//! libcma but errors on the mock).

use libcma_binding_rust::{Ledger, LedgerError, *};
use std::fs::OpenOptions;
use std::time::{SystemTime, UNIX_EPOCH};

/// Helper function to create a test token address
fn test_token_address() -> TokenAddress {
    let mut bytes = [0u8; 20];
    bytes[0] = 0x01;
    bytes[19] = 0xFF;
    TokenAddress::new(bytes)
}

/// Helper function to create a test account address
fn test_account_address() -> Address {
    let mut bytes = [0u8; 20];
    bytes[0] = 0xAA;
    bytes[19] = 0xBB;
    Address::new(bytes)
}

fn unique_temp_file_path() -> std::path::PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "libcma-ledger-test-{}-{}.bin",
        std::process::id(),
        unique
    ))
}

#[test]
fn test_ledger_initialization() {
    let ledger = Ledger::new();
    assert!(ledger.is_ok(), "Ledger initialization should succeed");
}

#[test]
fn test_ledger_reset() {
    let mut ledger = Ledger::new().expect("Failed to initialize ledger");
    let result = ledger.reset();
    assert!(result.is_ok(), "Ledger reset should succeed");
}

#[test]
fn test_init_from_file_reinitializes_ledger() {
    let mut ledger = Ledger::new().expect("Failed to initialize ledger");

    let token_addr = test_token_address();
    let token_id = U256::from_u64(99);
    ledger
        .retrieve_asset(
            None,
            Some(token_addr),
            Some(token_id),
            AssetType::TokenAddressId,
            RetrieveOperation::Create,
        )
        .expect("Should create asset before reinitializing");

    let path = unique_temp_file_path();
    let config = LedgerFileConfig::default();
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)
        .expect("Should create temp file");
    file.set_len(config.memory_length as u64)
        .expect("Should size temp file");

    ledger
        .init_from_file(&path, config)
        .expect("File-backed initialization should succeed");

    let result = ledger.retrieve_asset(
        None,
        Some(token_addr),
        Some(token_id),
        AssetType::TokenAddressId,
        RetrieveOperation::Find,
    );
    assert!(
        matches!(result, Err(LedgerError::AssetNotFound)),
        "Reinitialized ledger should not retain previous assets"
    );

    drop(ledger);
    std::fs::remove_file(path).expect("Should remove temp file");
}

#[test]
fn test_init_from_buffer_reinitializes_ledger() {
    let mut ledger = Ledger::new().expect("Failed to initialize ledger");

    let wallet_addr = test_account_address();
    ledger
        .retrieve_account(
            None,
            AccountType::WalletAddress,
            RetrieveOperation::Create,
            Some(wallet_addr.as_bytes()),
        )
        .expect("Should create account before reinitializing");

    let mut buffer = vec![0u8; 1024 * 1024];
    ledger
        .init_from_buffer(&mut buffer, LedgerBufferConfig::default())
        .expect("Buffer-backed initialization should succeed");

    let result = ledger.retrieve_account(
        None,
        AccountType::WalletAddress,
        RetrieveOperation::Find,
        Some(wallet_addr.as_bytes()),
    );
    assert!(
        matches!(result, Err(LedgerError::AccountNotFound)),
        "Reinitialized ledger should not retain previous accounts"
    );
}

#[test]
fn test_init_single_from_file_ether() {
    let mut ledger = Ledger::new().expect("Failed to initialize ledger");

    let path = unique_temp_file_path();
    let config = LedgerSingleFileConfig::default();
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)
        .expect("Should create temp file");
    file.set_len(config.memory_length as u64)
        .expect("Should size temp file");

    ledger
        .init_single_from_file(&path, config, LedgerAsset::Ether)
        .expect("Single-asset (ether) file-backed initialization should succeed");

    drop(ledger);
    std::fs::remove_file(path).expect("Should remove temp file");
}

#[test]
fn test_init_single_from_buffer_erc20() {
    let mut ledger = Ledger::new().expect("Failed to initialize ledger");

    let mut buffer = vec![0u8; 1024 * 1024];
    ledger
        .init_single_from_buffer(&mut buffer, 256, LedgerAsset::Erc20(test_token_address()))
        .expect("Single-asset (ERC-20) buffer-backed initialization should succeed");
}

#[test]
fn test_create_asset_by_token_address() {
    let mut ledger = Ledger::new().expect("Failed to initialize ledger");

    let token_addr = test_token_address();

    // Create a fungible (ERC-20) asset keyed by token address.
    let created = ledger
        .retrieve_asset(
            None, // No existing asset_id
            Some(token_addr),
            None,
            AssetType::TokenAddress,
            RetrieveOperation::Create,
        )
        .expect("Asset creation should succeed");

    // Backend-agnostic: the created asset must be findable again under the SAME id (the raw id
    // value differs between the mock (1-based) and real libcma (0-based), so don't assert `> 0`).
    let found = ledger
        .retrieve_asset(
            None,
            Some(token_addr),
            None,
            AssetType::TokenAddress,
            RetrieveOperation::Find,
        )
        .expect("Created asset should be findable");
    assert_eq!(
        created, found,
        "Find must return the id that Create assigned"
    );
}

#[test]
fn test_find_or_create_asset() {
    let mut ledger = Ledger::new().expect("Failed to initialize ledger");

    let token_addr = test_token_address();
    let token_id = U256::from_u64(2);

    // First call should create the asset
    let asset_id1 = ledger
        .retrieve_asset(
            None,
            Some(token_addr),
            Some(token_id),
            AssetType::TokenAddressId,
            RetrieveOperation::FindOrCreate,
        )
        .expect("Should create asset");

    // Second call with same parameters should find the existing asset
    let asset_id2 = ledger
        .retrieve_asset(
            None,
            Some(token_addr),
            Some(token_id),
            AssetType::TokenAddressId,
            RetrieveOperation::FindOrCreate,
        )
        .expect("Should find existing asset");

    assert_eq!(asset_id1, asset_id2, "Should return the same asset ID");
}

#[test]
fn test_create_account_by_wallet_address() {
    let mut ledger = Ledger::new().expect("Failed to initialize ledger");

    let wallet_addr = test_account_address();

    // Create an account using wallet address
    let created = ledger
        .retrieve_account(
            None, // No existing account_id
            AccountType::WalletAddress,
            RetrieveOperation::Create,
            Some(wallet_addr.as_slice()),
        )
        .expect("Account creation should succeed");

    // Backend-agnostic: the account must be findable again under the SAME id (real libcma is
    // 0-based, the mock 1-based — so assert the relationship, not a specific value).
    let found = ledger
        .retrieve_account(
            None,
            AccountType::WalletAddress,
            RetrieveOperation::Find,
            Some(wallet_addr.as_slice()),
        )
        .expect("Created account should be findable");
    assert_eq!(
        created, found,
        "Find must return the id that Create assigned"
    );
}

#[test]
fn test_find_or_create_account() {
    let mut ledger = Ledger::new().expect("Failed to initialize ledger");

    let wallet_addr = test_account_address();

    // First call should create the account
    let account_id1 = ledger
        .retrieve_account(
            None,
            AccountType::WalletAddress,
            RetrieveOperation::FindOrCreate,
            Some(wallet_addr.as_bytes()),
        )
        .expect("Should create account");

    // Second call with same address should find the existing account
    let account_id2 = ledger
        .retrieve_account(
            None,
            AccountType::WalletAddress,
            RetrieveOperation::FindOrCreate,
            Some(wallet_addr.as_bytes()),
        )
        .expect("Should find existing account");

    assert_eq!(
        account_id1, account_id2,
        "Should return the same account ID"
    );
}

#[test]
fn test_deposit_and_balance() {
    let mut ledger = Ledger::new().expect("Failed to initialize ledger");

    // Create a fungible asset
    let token_addr = test_token_address();
    let asset_id = ledger
        .retrieve_asset(
            None,
            Some(token_addr),
            None,
            AssetType::TokenAddress,
            RetrieveOperation::Create,
        )
        .expect("Should create asset");

    // Create an account
    let wallet_addr = test_account_address();
    let account_id = ledger
        .retrieve_account(
            None,
            AccountType::WalletAddress,
            RetrieveOperation::Create,
            Some(wallet_addr.as_slice()),
        )
        .expect("Should create account");

    // Check initial balance (should be zero)
    let initial_balance = ledger
        .get_balance(asset_id, account_id)
        .expect("Should get balance");
    assert_eq!(
        initial_balance,
        U256::zero(),
        "Initial balance should be zero"
    );

    // Deposit 1000 tokens
    let deposit_amount = U256::from_u64(1000);
    let result = ledger.deposit(asset_id, account_id, deposit_amount);
    assert!(result.is_ok(), "Deposit should succeed");

    // Check balance after deposit
    let balance = ledger
        .get_balance(asset_id, account_id)
        .expect("Should get balance");
    assert_eq!(
        balance, deposit_amount,
        "Balance should match deposit amount"
    );

    // Check total supply
    let total_supply = ledger
        .get_total_supply(asset_id)
        .expect("Should get total supply");
    assert_eq!(
        total_supply, deposit_amount,
        "Total supply should match deposit"
    );
}

#[test]
fn test_withdraw() {
    let mut ledger = Ledger::new().expect("Failed to initialize ledger");

    // Create fungible asset and account
    let token_addr = test_token_address();
    let asset_id = ledger
        .retrieve_asset(
            None,
            Some(token_addr),
            None,
            AssetType::TokenAddress,
            RetrieveOperation::Create,
        )
        .expect("Should create asset");

    let wallet_addr = test_account_address();
    let account_id = ledger
        .retrieve_account(
            None,
            AccountType::WalletAddress,
            RetrieveOperation::Create,
            Some(wallet_addr.as_slice()),
        )
        .expect("Should create account");

    // Deposit first
    let deposit_amount = U256::from_u64(5000);
    ledger
        .deposit(asset_id, account_id, deposit_amount)
        .expect("Deposit should succeed");

    // Withdraw 2000 tokens
    let withdraw_amount = U256::from_u64(2000);
    let result = ledger.withdraw(asset_id, account_id, withdraw_amount);
    assert!(result.is_ok(), "Withdraw should succeed");

    // Check balance
    let balance = ledger
        .get_balance(asset_id, account_id)
        .expect("Should get balance");
    let expected_balance = U256::from_u64(3000);
    assert_eq!(
        balance, expected_balance,
        "Balance should be reduced by withdrawal"
    );
}

#[test]
fn test_insufficient_funds_error() {
    let mut ledger = Ledger::new().expect("Failed to initialize ledger");

    // Fungible asset.
    let token_addr = test_token_address();
    let asset_id = ledger
        .retrieve_asset(
            None,
            Some(token_addr),
            None,
            AssetType::TokenAddress,
            RetrieveOperation::Create,
        )
        .expect("Should create asset");

    // Fund a FIRST account so the asset's total supply is non-zero. libcma checks the asset
    // supply for underflow BEFORE the per-account balance, so an empty account only reports
    // InsufficientFunds (rather than a supply underflow) once the supply itself can cover it.
    let mut funder_bytes = [0u8; 20];
    funder_bytes[0] = 0xF0;
    let funder = Address::new(funder_bytes);
    let funder_id = ledger
        .retrieve_account(
            None,
            AccountType::WalletAddress,
            RetrieveOperation::Create,
            Some(funder.as_slice()),
        )
        .expect("Should create funder account");
    ledger
        .deposit(asset_id, funder_id, U256::from_u64(1000))
        .expect("funder deposit should succeed");

    // A SECOND, empty account tries to withdraw more than its (zero) balance.
    let wallet_addr = test_account_address();
    let account_id = ledger
        .retrieve_account(
            None,
            AccountType::WalletAddress,
            RetrieveOperation::Create,
            Some(wallet_addr.as_slice()),
        )
        .expect("Should create account");

    let result = ledger.withdraw(asset_id, account_id, U256::from_u64(100));
    assert!(
        result.is_err(),
        "Withdraw from an empty account should fail"
    );
    match result.unwrap_err() {
        LedgerError::InsufficientFunds => {
            // Expected error
        }
        e => panic!("Expected InsufficientFunds error, got {:?}", e),
    }
}

#[test]
fn test_transfer() {
    let mut ledger = Ledger::new().expect("Failed to initialize ledger");

    // Create a fungible asset
    let token_addr = test_token_address();
    let asset_id = ledger
        .retrieve_asset(
            None,
            Some(token_addr),
            None,
            AssetType::TokenAddress,
            RetrieveOperation::Create,
        )
        .expect("Should create asset");

    // Create two accounts
    let mut wallet1_bytes = [0u8; 20];
    wallet1_bytes[0] = 0x11;
    let wallet1 = Address::new(wallet1_bytes);
    let account1 = ledger
        .retrieve_account(
            None,
            AccountType::WalletAddress,
            RetrieveOperation::Create,
            Some(wallet1.as_slice()),
        )
        .expect("Should create account 1");

    let mut wallet2_bytes = [0u8; 20];
    wallet2_bytes[0] = 0x22;
    let wallet2 = Address::new(wallet2_bytes);
    let account2 = ledger
        .retrieve_account(
            None,
            AccountType::WalletAddress,
            RetrieveOperation::Create,
            Some(wallet2.as_slice()),
        )
        .expect("Should create account 2");

    // Deposit to account1
    let deposit_amount = U256::from_u64(10000);
    ledger
        .deposit(asset_id, account1, deposit_amount)
        .expect("Deposit should succeed");

    // Transfer 3000 from account1 to account2
    let transfer_amount = U256::from_u64(3000);
    let result = ledger.transfer(asset_id, account1, account2, transfer_amount);
    assert!(result.is_ok(), "Transfer should succeed");

    // Check balances
    let balance1 = ledger
        .get_balance(asset_id, account1)
        .expect("Should get balance 1");
    let balance2 = ledger
        .get_balance(asset_id, account2)
        .expect("Should get balance 2");

    let expected_balance1 = U256::from_u64(7000);
    assert_eq!(
        balance1, expected_balance1,
        "Account 1 balance should be reduced"
    );
    assert_eq!(
        balance2, transfer_amount,
        "Account 2 balance should match transfer amount"
    );

    // Total supply should remain the same
    let total_supply = ledger
        .get_total_supply(asset_id)
        .expect("Should get total supply");
    assert_eq!(
        total_supply, deposit_amount,
        "Total supply should remain unchanged"
    );
}

#[test]
fn test_multiple_assets_and_accounts() {
    let mut ledger = Ledger::new().expect("Failed to initialize ledger");

    // Create two different fungible assets, keyed by two distinct token addresses.
    let mut token1_bytes = [0u8; 20];
    token1_bytes[0] = 0xA1;
    let token1_addr = TokenAddress::new(token1_bytes);
    let asset1 = ledger
        .retrieve_asset(
            None,
            Some(token1_addr),
            None,
            AssetType::TokenAddress,
            RetrieveOperation::Create,
        )
        .expect("Should create asset 1");

    let mut token2_bytes = [0u8; 20];
    token2_bytes[0] = 0xB2;
    let token2_addr = TokenAddress::new(token2_bytes);
    let asset2 = ledger
        .retrieve_asset(
            None,
            Some(token2_addr),
            None,
            AssetType::TokenAddress,
            RetrieveOperation::Create,
        )
        .expect("Should create asset 2");

    // Create an account
    let wallet_addr = test_account_address();
    let account_id = ledger
        .retrieve_account(
            None,
            AccountType::WalletAddress,
            RetrieveOperation::Create,
            Some(wallet_addr.as_slice()),
        )
        .expect("Should create account");

    // Deposit different amounts to the same account for different assets
    let amount1 = U256::from_u64(100);
    let amount2 = U256::from_u64(200);

    ledger
        .deposit(asset1, account_id, amount1)
        .expect("Deposit asset1 should succeed");
    ledger
        .deposit(asset2, account_id, amount2)
        .expect("Deposit asset2 should succeed");

    // Check balances are independent
    let balance1 = ledger
        .get_balance(asset1, account_id)
        .expect("Should get balance 1");
    let balance2 = ledger
        .get_balance(asset2, account_id)
        .expect("Should get balance 2");

    assert_eq!(balance1, amount1, "Balance 1 should be correct");
    assert_eq!(balance2, amount2, "Balance 2 should be correct");

    // Check total supplies
    let supply1 = ledger
        .get_total_supply(asset1)
        .expect("Should get supply 1");
    let supply2 = ledger
        .get_total_supply(asset2)
        .expect("Should get supply 2");

    assert_eq!(supply1, amount1, "Supply 1 should be correct");
    assert_eq!(supply2, amount2, "Supply 2 should be correct");
}

#[test]
fn test_account_not_found_error() {
    let mut ledger = Ledger::new().expect("Failed to initialize ledger");

    // Create a fungible asset.
    let token_addr = test_token_address();
    let asset_id = ledger
        .retrieve_asset(
            None,
            Some(token_addr),
            None,
            AssetType::TokenAddress,
            RetrieveOperation::Create,
        )
        .expect("Should create asset");

    // An OPERATION against a non-existent account must fail with AccountNotFound. (`get_balance`
    // is NOT used here: real libcma returns 0 for an unknown (asset, account) pair rather than
    // erroring — it is the mutating paths that validate account existence.)
    let fake_account_id = LedgerAccountId(99999);
    let result = ledger.deposit(asset_id, fake_account_id, U256::from_u64(1));

    assert!(result.is_err(), "Should fail for non-existent account");
    match result.unwrap_err() {
        LedgerError::AccountNotFound => {
            // Expected error
        }
        e => panic!("Expected AccountNotFound error, got {:?}", e),
    }
}

#[test]
fn test_asset_not_found_error() {
    let mut ledger = Ledger::new().expect("Failed to initialize ledger");

    // Create account
    let wallet_addr = test_account_address();
    let account_id = ledger
        .retrieve_account(
            None,
            AccountType::WalletAddress,
            RetrieveOperation::Create,
            Some(wallet_addr.as_slice()),
        )
        .expect("Should create account");

    // An OPERATION against a non-existent asset must fail with AssetNotFound (again via a
    // mutating path, not `get_balance`, which returns 0 for unknown pairs on real libcma).
    let fake_asset_id = LedgerAssetId(99999);
    let result = ledger.deposit(fake_asset_id, account_id, U256::from_u64(1));

    assert!(result.is_err(), "Should fail for non-existent asset");
    match result.unwrap_err() {
        LedgerError::AssetNotFound => {
            // Expected error
        }
        e => panic!("Expected AssetNotFound error, got {:?}", e),
    }
}

#[test]
fn test_find_nonexistent_asset() {
    let mut ledger = Ledger::new().expect("Failed to initialize ledger");

    let token_addr = test_token_address();
    let token_id = U256::from_u64(600);

    // Try to find an asset that doesn't exist (without creating)
    let result = ledger.retrieve_asset(
        None,
        Some(token_addr),
        Some(token_id),
        AssetType::TokenAddressId,
        RetrieveOperation::Find, // Only find, don't create
    );

    assert!(result.is_err(), "Should fail to find non-existent asset");
    match result.unwrap_err() {
        LedgerError::AssetNotFound => {
            // Expected error
        }
        e => panic!("Expected AssetNotFound error, got {:?}", e),
    }
}

#[test]
fn test_find_nonexistent_account() {
    let mut ledger = Ledger::new().expect("Failed to initialize ledger");

    let wallet_addr = test_account_address();

    // Try to find an account that doesn't exist (without creating)
    let result = ledger.retrieve_account(
        None,
        AccountType::WalletAddress,
        RetrieveOperation::Find, // Only find, don't create
        Some(wallet_addr.as_bytes()),
    );

    assert!(result.is_err(), "Should fail to find non-existent account");
    match result.unwrap_err() {
        LedgerError::AccountNotFound => {
            // Expected error
        }
        e => panic!("Expected AccountNotFound error, got {:?}", e),
    }
}

#[test]
fn test_retrieve_ether_asset() {
    // The Base (ether) asset type is only supported by the buffer/file-backed multi-asset ledger
    // (`cma_ledger_memory`). The transient `Ledger::new()` backend (`cma_ledger_basic`) has no
    // Base case and returns EINVAL, so back the ledger with a buffer before touching ether.
    let mut ledger = Ledger::new().expect("Failed to initialize ledger");
    let mut buffer = vec![0u8; 1024 * 1024];
    ledger
        .init_from_buffer(&mut buffer, LedgerBufferConfig::default())
        .expect("Buffer-backed initialization should succeed");

    let created = ledger
        .retrieve_ether_assets()
        .expect("Should create base ether asset");
    // Backend-agnostic: retrieving it again (find-or-create) must return the same id.
    let again = ledger
        .retrieve_ether_assets()
        .expect("Should find the existing base ether asset");
    assert_eq!(created, again, "Base ether asset id must be stable");
}

/// The NON-FUNGIBLE (`TokenAddressId`) asset type is capped at supply 1: a single deposit of
/// exactly 1 succeeds, and anything else (a second unit, or an initial amount > 1) is rejected
/// with `SupplyOverflow`. This is real libcma behaviour the mock does not model, so it only runs
/// against a real backend.
#[cfg(any(feature = "host-real", feature = "riscv64"))]
#[test]
fn test_nft_asset_deposit_is_capped_at_one() {
    let mut ledger = Ledger::new().expect("Failed to initialize ledger");

    let token_addr = test_token_address();
    let token_id = U256::from_u64(42);
    let asset_id = ledger
        .retrieve_asset(
            None,
            Some(token_addr),
            Some(token_id),
            AssetType::TokenAddressId,
            RetrieveOperation::Create,
        )
        .expect("Should create NFT asset");

    let holder = ledger
        .retrieve_account(
            None,
            AccountType::WalletAddress,
            RetrieveOperation::Create,
            Some(test_account_address().as_slice()),
        )
        .expect("Should create holder account");

    // Depositing more than one unit of a unique token is rejected.
    match ledger.deposit(asset_id, holder, U256::from_u64(5)) {
        Err(LedgerError::SupplyOverflow) => {}
        other => panic!("NFT deposit > 1 should be SupplyOverflow, got {:?}", other),
    }

    // Minting the single unit succeeds...
    ledger
        .deposit(asset_id, holder, U256::from_u64(1))
        .expect("Minting the single NFT unit should succeed");
    assert_eq!(
        ledger.get_total_supply(asset_id).expect("supply"),
        U256::from_u64(1),
        "NFT supply must be exactly 1"
    );

    // ...but a second unit pushes supply past 1 and is rejected.
    match ledger.deposit(asset_id, holder, U256::from_u64(1)) {
        Err(LedgerError::SupplyOverflow) => {}
        other => panic!("second NFT unit should be SupplyOverflow, got {:?}", other),
    }
}

#[test]
fn test_large_amounts() {
    let mut ledger = Ledger::new().expect("Failed to initialize ledger");

    // Create a fungible asset and account
    let token_addr = test_token_address();
    let asset_id = ledger
        .retrieve_asset(
            None,
            Some(token_addr),
            None,
            AssetType::TokenAddress,
            RetrieveOperation::Create,
        )
        .expect("Should create asset");

    let wallet_addr = test_account_address();
    let account_id = ledger
        .retrieve_account(
            None,
            AccountType::WalletAddress,
            RetrieveOperation::Create,
            Some(wallet_addr.as_slice()),
        )
        .expect("Should create account");

    // Test with a large u64 value
    let large_amount = U256::from_u64(u64::MAX);
    ledger
        .deposit(asset_id, account_id, large_amount)
        .expect("Should handle large amounts");

    let balance = ledger
        .get_balance(asset_id, account_id)
        .expect("Should get balance");
    assert_eq!(
        balance, large_amount,
        "Large amount should be handled correctly"
    );
}
