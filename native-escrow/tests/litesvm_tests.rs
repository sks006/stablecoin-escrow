// Step 1: Import LiteSVM test ledger simulation environment from litesvm crate.
use litesvm::LiteSVM;
// Step 2: Import the core Solana SDK Account structure representing account state.
use solana_sdk::account::Account;
// Step 3: Import AccountMeta and Instruction structures for transaction building.
use solana_sdk::instruction::{AccountMeta, Instruction};
// Step 4: Import the Pack trait boundary required to serialize and deserialize token state structures.
use solana_sdk::program_pack::Pack;
// Step 5: Import Pubkey type representing 32-byte public keys.
use solana_sdk::pubkey::Pubkey;
// Step 6: Import Keypair and Signer traits for keypair generation and signing.
use solana_sdk::signature::{Keypair, Signer};
// Step 7: Import Rent structure representing the Solana network rent-exemption settings.
use solana_sdk::sysvar::rent::Rent;
// Step 8: Import Transaction structure to package instructions and submit them to LiteSVM.
use solana_sdk::transaction::Transaction;
// Step 9: Import TokenAccount and Mint structure layouts from the spl-token-2022 crate.
use spl_token_2022::state::{Account as TokenAccount, Mint};

// ── Token helpers ────────────────────────────────────────────────
// Step 10: Define a helper function to pack a Mint structure into serialized byte format.
fn pack_mint(authority: &Pubkey, decimals: u8) -> Vec<u8> {
    // Step 11: Pre-allocate a byte vector of size Mint::LEN.
    let mut buf = vec![0u8; Mint::LEN];
    // Step 12: Pack the Mint struct representation into the byte buffer.
    Mint::pack(
        Mint {
            // Step 13: Set the mint authority Pubkey.
            mint_authority: Some(*authority).into(),
            // Step 14: Set the total supply of tokens.
            supply: 1_000_000_000_000,
            // Step 15: Set the decimal precision.
            decimals,
            // Step 16: Mark the mint as initialized.
            is_initialized: true,
            // Step 17: Disable the freeze authority.
            freeze_authority: None.into(),
        },
        &mut buf,
    ).unwrap();
    // Step 18: Return the packed byte vector.
    buf
}

// Step 19: Define a helper function to pack a TokenAccount structure into serialized byte format.
fn pack_token_account(mint: &Pubkey, owner: &Pubkey, amount: u64) -> Vec<u8> {
    // Step 20: Pre-allocate a byte vector of size TokenAccount::LEN.
    let mut buf = vec![0u8; TokenAccount::LEN];
    // Step 21: Pack the TokenAccount struct representation into the byte buffer.
    TokenAccount::pack(
        TokenAccount {
            // Step 22: Associate the token account with the mint.
            mint: *mint,
            // Step 23: Assign the owner of the token account.
            owner: *owner,
            // Step 24: Set the initial token balance of the account.
            amount,
            // Step 25: Disable delegate authority.
            delegate: None.into(),
            // Step 26: Mark the account state as Initialized.
            state: spl_token_2022::state::AccountState::Initialized,
            // Step 27: Disable native token option.
            is_native: None.into(),
            // Step 28: Set the delegated amount to zero.
            delegated_amount: 0,
            // Step 29: Disable close authority.
            close_authority: None.into(),
        },
        &mut buf,
    ).unwrap();
    // Step 30: Return the packed byte vector.
    buf
}

// Step 31: Define a helper function to allocate and fund a standard system account in LiteSVM.
fn create_funded_account(svm: &mut LiteSVM, key: &Pubkey, lamports: u64) {
    // Step 32: Populate account state directly in LiteSVM.
    svm.set_account(
        // Step 33: Pass the Pubkey of the account by value.
        *key,
        // Step 34: Construct the Account struct.
        Account {
            // Step 35: Set the lamport balance.
            lamports,
            // Step 36: Initialize data as empty.
            data: vec![],
            // Step 37: Set the owner to System Program ID.
            owner: solana_sdk::system_program::ID,
            // Step 38: Set executable flag to false.
            executable: false,
            // Step 39: Set rent epoch to zero.
            rent_epoch: 0,
        },
    );
}

// Step 40: Declare the full flow integration test using the #[test] attribute.
#[test]
fn test_native_escrow_full_flow() {
    // Step 41: Instantiate a new LiteSVM ledger environment.
    let mut svm = LiteSVM::new();
    
    // Step 42: Generate a random program ID for the deployed escrow program.
    let program_id = Pubkey::new_unique();

    // Step 43: Load compiled BPF/SBF binary from deploy folder.
    let elf = std::fs::read("../target/deploy/native_escrow.so")
        .expect("Run 'cargo build-sbf' inside native-escrow/ first");
        
    // Step 44: Add the program to the LiteSVM runtime state.
    svm.add_program(program_id, &elf);

    // Step 45: Generate Keypair for the maker.
    let maker = Keypair::new();
    // Step 46: Generate Keypair for the taker.
    let taker = Keypair::new();
    // Step 47: Generate Keypair for the cancel authority.
    let cancel_authority = Keypair::new();
    // Step 48: Generate Keypair for the mint.
    let mint = Keypair::new();
    // Step 49: Generate Keypair for the maker's token account.
    let maker_token = Keypair::new();
    // Step 50: Generate Keypair for the taker's token account.
    let taker_token = Keypair::new();
    // Step 51: Generate Keypair for the maker_ref rent destination.
    let maker_ref = Keypair::new();

    // Step 52: Fund the maker's account with 100 SOL.
    create_funded_account(&mut svm, &maker.pubkey(), 100_000_000_000);
    // Step 53: Fund the maker_ref account with 10 SOL.
    create_funded_account(&mut svm, &maker_ref.pubkey(), 10_000_000_000);
    // Step 54: Fund the taker's account with 10 SOL so they can cover transaction fees.
    create_funded_account(&mut svm, &taker.pubkey(), 10_000_000_000);

    // Step 55: Pack mint state with 6 decimals.
    let mint_data = pack_mint(&maker.pubkey(), 6);
    // Step 56: Calculate rent exemption lamports for the mint.
    let rent = Rent::default().minimum_balance(mint_data.len());
    // Step 57: Set the mint account state in LiteSVM.
    svm.set_account(mint.pubkey(), Account {
        lamports: rent,
        data: mint_data,
        owner: spl_token_2022::ID,
        executable: false,
        rent_epoch: 0,
    });

    // Step 58: Pack the maker token account with 1000 tokens (1_000_000_000 raw amount due to 6 decimals).
    let maker_token_data = pack_token_account(&mint.pubkey(), &maker.pubkey(), 1_000_000_000);
    // Step 59: Calculate rent exemption lamports for the token account.
    let token_rent = Rent::default().minimum_balance(maker_token_data.len());
    // Step 60: Set the maker token account state in LiteSVM.
    svm.set_account(maker_token.pubkey(), Account {
        lamports: token_rent,
        data: maker_token_data,
        owner: spl_token_2022::ID,
        executable: false,
        rent_epoch: 0,
    });

    // Step 61: Pack the taker token account with 0 tokens.
    let taker_token_data = pack_token_account(&mint.pubkey(), &taker.pubkey(), 0);
    // Step 62: Set the taker token account state in LiteSVM.
    svm.set_account(taker_token.pubkey(), Account {
        lamports: Rent::default().minimum_balance(taker_token_data.len()),
        data: taker_token_data,
        owner: spl_token_2022::ID,
        executable: false,
        rent_epoch: 0,
    });

    // Step 63: Derive the Escrow PDA.
    let (escrow_pda, bump) = Pubkey::find_program_address(
        &[b"escrow", maker.pubkey().as_ref(), taker.pubkey().as_ref(), mint.pubkey().as_ref()],
        &program_id,
    );
    // Step 64: Derive the Vault PDA.
    let (vault_pda, _) = Pubkey::find_program_address(
        &[b"vault", escrow_pda.as_ref()],
        &program_id,
    );
    // Step 65: Pack the vault token account with 0 tokens.
    let vault_token_data = pack_token_account(&mint.pubkey(), &escrow_pda, 0);
    // Step 66: Set the vault account state in LiteSVM (pre-allocated).
    svm.set_account(vault_pda, Account {
        lamports: Rent::default().minimum_balance(vault_token_data.len()),
        data: vault_token_data,
        owner: spl_token_2022::ID,
        executable: false,
        rent_epoch: 0,
    });

    // Step 67: Set the escrow amount to 500 tokens (500_000_000 raw).
    let amount = 500_000_000u64;           
    // Step 68: Set the transfer fee to 50 BPS (0.5%).
    let transfer_fee_bps = 50u16;          
    // Step 69: Initialize the allowlist root as 32 empty bytes.
    let allowlist_root = [0u8; 32];
    // Step 70: Initialize the blacklist root as 32 empty bytes.
    let blacklist_root = [0u8; 32];

    // Step 71: Prepare the serialized Initialize instruction payload (107 bytes total).
    let mut data = vec![0u8]; // Discriminator: 0 (Initialize)
    // Step 72: Add the amount parameter (8 bytes).
    data.extend_from_slice(&amount.to_le_bytes());
    // Step 73: Add the transfer fee bps parameter (2 bytes).
    data.extend_from_slice(&transfer_fee_bps.to_le_bytes());
    // Step 74: Add the taker public key parameter (32 bytes).
    data.extend_from_slice(taker.pubkey().as_ref());
    // Step 75: Add the allowlist Merkle root (32 bytes).
    data.extend_from_slice(&allowlist_root);
    // Step 76: Add the blacklist Merkle root (32 bytes).
    data.extend_from_slice(&blacklist_root);

    // Step 77: Construct the Initialize instruction.
    let init_ix = Instruction {
        program_id: program_id,
        accounts: vec![
            AccountMeta::new(maker.pubkey(), true),
            AccountMeta::new_readonly(mint.pubkey(), false),
            AccountMeta::new(maker_token.pubkey(), false),
            AccountMeta::new(escrow_pda, false),
            AccountMeta::new(vault_pda, false),
            AccountMeta::new_readonly(solana_sdk::system_program::ID, false),
            AccountMeta::new_readonly(spl_token_2022::ID, false),
        ],
        data,
    };

    // Step 78: Create transaction signed by maker.
    let tx = Transaction::new_signed_with_payer(
        &[init_ix],
        Some(&maker.pubkey()),
        &[&maker],
        svm.latest_blockhash(),
    );
    
    // Step 79: Execute transaction on LiteSVM.
    let result = svm.send_transaction(tx).expect("Initialize failed");
    // Step 80: Log the execution result.
    println!("✅ Initialize Execution State: {:?}", result);

    // Step 81: Verify that the escrow state account was created.
    let escrow_account = svm.get_account(&escrow_pda).unwrap();
    // Step 82: Ensure the escrow account owner is set to the program ID.
    assert_eq!(escrow_account.owner, program_id);
    // Step 83: Print verification success.
    println!("✅ Escrow state created");

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // PHASE 2: EXECUTE TRANSFER (Settlement & Teardown)
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    // Step 84: Prepare the ExecuteTransfer instruction payload (Discriminator: 1).
    let execute_data = vec![1u8];

    // Step 85: Construct the ExecuteTransfer instruction.
    let execute_ix = Instruction {
        program_id: program_id,
        accounts: vec![
            AccountMeta::new(taker.pubkey(), true),          
            AccountMeta::new(escrow_pda, false),             
            AccountMeta::new(vault_pda, false),              
            AccountMeta::new(taker_token.pubkey(), false),   
            AccountMeta::new(maker_token.pubkey(), false),   
            AccountMeta::new(maker_ref.pubkey(), false),     
            AccountMeta::new_readonly(mint.pubkey(), false), 
            AccountMeta::new_readonly(spl_token_2022::ID, false), 
        ],
        data: execute_data,
    };

    // Step 86: Fetch the taker's lamport balance before settlement execution.
    let balance_before = svm.get_account(&taker.pubkey()).unwrap().lamports;

    // Step 87: Create settlement transaction signed by taker.
    let execute_tx = Transaction::new_signed_with_payer(
        &[execute_ix],
        Some(&taker.pubkey()),
        &[&taker],
        svm.latest_blockhash(),
    );
    // Step 88: Submit settlement transaction to LiteSVM.
    let exec_result = svm.send_transaction(execute_tx).expect("ExecuteTransfer failed");

    // Step 89: Print transaction logs for tracing CPIs.
    println!("\n📋 ExecuteTransfer Logs:");
    for log in &exec_result.logs {
        println!("  {}", log);
    }

    // Step 90: Fetch the taker's lamport balance after settlement execution.
    let balance_after = svm.get_account(&taker.pubkey()).unwrap().lamports;
    
    // Step 91: Calculate true lamports consumed by the transaction.
    let total_lamport_cost = balance_before.saturating_sub(balance_after);

    // Step 92: Retrieve the number of compute units consumed.
    let consumed = exec_result.compute_units_consumed;
    // Step 93: Set the default limit of 200k compute units.
    let allocated = 200_000u64;
    // Step 94: Calculate compute usage percentage.
    let percentage = (consumed as f64 / allocated as f64) * 100.0;

    // Step 95: Log benchmark results to console.
    println!("\n🔥🚀 ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ 🚀🔥");
    println!("⚡ 🟢 [NATIVE BENCHMARK] ExecuteTransfer");
    println!("─────────────────────────────────────────────────────────────");
    println!("🟢 CU Used:        {} / {} CUs", consumed, allocated);
    println!("📊 Usage:         {:.1}%", percentage);
    println!("💰 Ledger Cost:    {} lamports", total_lamport_cost);
    println!("🔥🚀 ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ 🚀🔥\n");

    // Step 96: Get the escrow account state after settlement.
    let escrow_post = svm.get_account(&escrow_pda);
    // Step 97: Assert that the escrow account is closed and has 0 lamports.
    assert!(
        escrow_post.is_none() || escrow_post.unwrap().lamports == 0,
        "FATAL: Escrow PDA lamports not drained"
    );

    // Step 98: Get the vault account state after settlement.
    let vault_post = svm.get_account(&vault_pda);
    // Step 99: Assert that the vault account is closed and has 0 lamports.
    assert!(
        vault_post.is_none() || vault_post.unwrap().lamports == 0,
        "FATAL: Vault PDA lamports not drained"
    );
    // Step 100: Print cleanup success.
    println!("✅ Escrow & Vault rent deposits successfully reclaimed");
    // Step 101: Print flow completion.
    println!("\n=== FULL FLOW BENCHMARK COMPLETE ===");
}

// Step 102: Declare fast-path integration test verifying zero-fee optimization.
#[test]
fn test_native_escrow_zero_fee_fast_path() {
    // Step 103: Instantiate a new LiteSVM ledger environment.
    let mut svm = LiteSVM::new();
    // Step 104: Generate a random program ID for the escrow program.
    let program_id = Pubkey::new_unique();

    // Step 105: Read compiled SBF binary from target directory.
    let elf = std::fs::read("../target/deploy/native_escrow.so")
        .expect("Run 'cargo build-sbf' inside native-escrow/ first");
    // Step 106: Deploy program to the test environment.
    svm.add_program(program_id, &elf);

    // Step 107: Generate Keypairs for identities.
    let maker = Keypair::new();
    let taker = Keypair::new();
    let mint = Keypair::new();
    let maker_token = Keypair::new();
    let taker_token = Keypair::new();
    let maker_ref = Keypair::new();

    // Step 108: Fund accounts with SOL.
    create_funded_account(&mut svm, &maker.pubkey(), 100_000_000_000);
    create_funded_account(&mut svm, &maker_ref.pubkey(), 10_000_000_000);
    create_funded_account(&mut svm, &taker.pubkey(), 10_000_000_000);

    // Step 109: Pack mint state.
    let mint_data = pack_mint(&maker.pubkey(), 6);
    // Step 110: Calculate rent exemption lamports for the mint.
    let rent = Rent::default().minimum_balance(mint_data.len());
    // Step 111: Set mint account state in LiteSVM.
    svm.set_account(mint.pubkey(), Account {
        lamports: rent,
        data: mint_data,
        owner: spl_token_2022::ID,
        executable: false,
        rent_epoch: 0,
    });

    // Step 112: Pack maker token account with 1000 tokens.
    let maker_token_data = pack_token_account(&mint.pubkey(), &maker.pubkey(), 1_000_000_000);
    // Step 113: Calculate rent exemption lamports for the token account.
    let token_rent = Rent::default().minimum_balance(maker_token_data.len());
    // Step 114: Set maker token account state in LiteSVM.
    svm.set_account(maker_token.pubkey(), Account {
        lamports: token_rent,
        data: maker_token_data,
        owner: spl_token_2022::ID,
        executable: false,
        rent_epoch: 0,
    });

    // Step 115: Pack taker token account with 0 tokens.
    let taker_token_data = pack_token_account(&mint.pubkey(), &taker.pubkey(), 0);
    // Step 116: Set taker token account state in LiteSVM.
    svm.set_account(taker_token.pubkey(), Account {
        lamports: Rent::default().minimum_balance(taker_token_data.len()),
        data: taker_token_data,
        owner: spl_token_2022::ID,
        executable: false,
        rent_epoch: 0,
    });

    // Step 117: Derive Escrow PDA.
    let (escrow_pda, _) = Pubkey::find_program_address(
        &[b"escrow", maker.pubkey().as_ref(), taker.pubkey().as_ref(), mint.pubkey().as_ref()],
        &program_id,
    );
    // Step 118: Derive Vault PDA.
    let (vault_pda, _) = Pubkey::find_program_address(
        &[b"vault", escrow_pda.as_ref()],
        &program_id,
    );
    // Step 119: Pack vault token account.
    let vault_token_data = pack_token_account(&mint.pubkey(), &escrow_pda, 0);
    // Step 120: Set vault account state in LiteSVM.
    svm.set_account(vault_pda, Account {
        lamports: Rent::default().minimum_balance(vault_token_data.len()),
        data: vault_token_data,
        owner: spl_token_2022::ID,
        executable: false,
        rent_epoch: 0,
    });

    // Step 121: Set amount to 500 tokens (500_000_000 raw).
    let amount = 500_000_000u64;
    // Step 122: Set fee BPS to 0 to trigger the optimized zero-fee fast path.
    let transfer_fee_bps = 0u16;
    // Step 123: Initialize allowlist root.
    let allowlist_root = [0u8; 32];
    // Step 124: Initialize blacklist root.
    let blacklist_root = [0u8; 32];

    // Step 125: Prepare serialized Initialize instruction data payload.
    let mut data = vec![0u8];
    data.extend_from_slice(&amount.to_le_bytes());
    data.extend_from_slice(&transfer_fee_bps.to_le_bytes());
    data.extend_from_slice(taker.pubkey().as_ref());
    data.extend_from_slice(&allowlist_root);
    data.extend_from_slice(&blacklist_root);

    // Step 126: Construct the Initialize instruction.
    let init_ix = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(maker.pubkey(), true),
            AccountMeta::new_readonly(mint.pubkey(), false),
            AccountMeta::new(maker_token.pubkey(), false),
            AccountMeta::new(escrow_pda, false),
            AccountMeta::new(vault_pda, false),
            AccountMeta::new_readonly(solana_sdk::system_program::ID, false),
            AccountMeta::new_readonly(spl_token_2022::ID, false),
        ],
        data,
    };

    // Step 127: Create transaction signed by maker.
    let tx = Transaction::new_signed_with_payer(
        &[init_ix],
        Some(&maker.pubkey()),
        &[&maker],
        svm.latest_blockhash(),
    );
    // Step 128: Submit transaction to LiteSVM.
    svm.send_transaction(tx).expect("Initialize failed");

    // Step 129: Construct ExecuteTransfer instruction for zero-fee fast path (Discriminator: 1).
    let execute_ix = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(taker.pubkey(), true),
            AccountMeta::new(escrow_pda, false),
            AccountMeta::new(vault_pda, false),
            AccountMeta::new(taker_token.pubkey(), false),
            AccountMeta::new(maker_token.pubkey(), false),
            AccountMeta::new(maker_ref.pubkey(), false),
            AccountMeta::new_readonly(mint.pubkey(), false),
            AccountMeta::new_readonly(spl_token_2022::ID, false),
        ],
        data: vec![1u8],
    };

    // Step 130: Fetch the taker's lamport balance before settlement execution.
    let balance_before = svm.get_account(&taker.pubkey()).unwrap().lamports;

    // Step 131: Create settlement transaction signed by taker.
    let execute_tx = Transaction::new_signed_with_payer(
        &[execute_ix],
        Some(&taker.pubkey()),
        &[&taker],
        svm.latest_blockhash(),
    );
    // Step 132: Submit transaction to LiteSVM.
    let exec_result = svm.send_transaction(execute_tx).expect("ExecuteTransfer (zero-fee) failed");

    // Step 133: Print transaction execution logs.
    println!("\n📋 ExecuteTransfer (ZERO-FEE) Logs:");
    for log in &exec_result.logs {
        println!("  {}", log);
    }

    // Step 134: Fetch the taker's lamport balance after settlement execution.
    let balance_after = svm.get_account(&taker.pubkey()).unwrap().lamports;
    // Step 135: Calculate true lamports consumed by the transaction.
    let total_lamport_cost = balance_before.saturating_sub(balance_after);

    // Step 136: Retrieve the number of compute units consumed.
    let consumed = exec_result.compute_units_consumed;
    // Step 137: Set the default limit of 200k compute units.
    let allocated = 200_000u64;
    // Step 138: Calculate compute usage percentage.
    let percentage = (consumed as f64 / allocated as f64) * 100.0;

    // Step 139: Define Anchor baseline Compute Unit value for comparison.
    let anchor_baseline = 18_183u64;
    // Step 140: Define native baseline Compute Unit value for comparison.
    let native_baseline = 33_840u64;
    // Step 141: Calculate savings percentage relative to Anchor.
    let reduction_vs_anchor = ((anchor_baseline as f64 - consumed as f64) / anchor_baseline as f64) * 100.0;
    // Step 142: Calculate savings percentage relative to initial native escrow implementation.
    let reduction_vs_native = ((native_baseline as f64 - consumed as f64) / native_baseline as f64) * 100.0;

    // Step 143: Log zero-fee benchmark results to console.
    println!("\n🔥🚀 ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ 🚀🔥");
    println!("⚡ 🟢 [L3 ZERO-FEE PATH] ExecuteTransfer");
    println!("─────────────────────────────────────────────────────────────");
    println!("🟢 CU Used:        {} / {} CUs", consumed, allocated);
    println!("📊 Usage:         {:.1}%", percentage);
    println!("💰 Ledger Cost:    {} lamports", total_lamport_cost);
    println!("─────────────────────────────────────────────────────────────");
    println!("📉 vs Anchor (18,183):     {:+.1}%", -reduction_vs_anchor);
    println!("📉 vs Native v0 (33,840):  {:+.1}%", -reduction_vs_native);
    println!("🔥🚀 ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ 🚀🔥\n");

    // Step 144: Fetch the escrow account state.
    let escrow_post = svm.get_account(&escrow_pda);
    // Step 145: Assert that the escrow account is closed and has 0 lamports.
    assert!(
        escrow_post.is_none() || escrow_post.unwrap().lamports == 0,
        "FATAL: Escrow PDA lamports not drained"
    );
    // Step 146: Fetch the vault account state.
    let vault_post = svm.get_account(&vault_pda);
    // Step 147: Assert that the vault account is closed and has 0 lamports.
    assert!(
        vault_post.is_none() || vault_post.unwrap().lamports == 0,
        "FATAL: Vault PDA lamports not drained"
    );
    // Step 148: Print cleanup success.
    println!("✅ Zero-fee path: Escrow & Vault reclaimed");
}