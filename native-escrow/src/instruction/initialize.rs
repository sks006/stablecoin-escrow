// Step 1: Import AccountInfo representing the metadata and pointers for Solana accounts.
use solana_program::account_info::AccountInfo;
// Step 2: Import ProgramResult representing the success/failure state of execution.
use solana_program::entrypoint::ProgramResult;
// Step 3: Import msg! macro placeholder/helper for program logging.
use solana_program::msg;
// Step 4: Import invoke (standard CPI) and invoke_signed (CPI with PDA signer seeds).
use solana_program::program::{ invoke, invoke_signed };
// Step 5: Import ProgramError representing system/runtime errors.
use solana_program::program_error::ProgramError;
// Step 6: Import Pubkey structure for identifying Solana accounts.
use solana_program::pubkey::Pubkey;
// Step 7: Import Rent structure for calculation of rent exemption minimum balance.
use solana_program::rent::Rent;
// Step 8: Import Sysvar trait to access system variables from the Solana runtime.
use solana_program::sysvar::Sysvar;
// Step 9: Import Instruction struct to manually build Cross-Program Invocations (CPIs).
use solana_program::instruction::Instruction;
// Step 10: Import AccountMeta to configure account permissions for custom instructions.
use solana_program::instruction::AccountMeta;
// Step 11: Import try_from_bytes_mut to perform zero-copy mutable casting on raw data slices.
use bytemuck::try_from_bytes_mut;
// Step 12: Import custom EscrowError and NativeEscrowState layout from the local crate.
use crate::{ errors::EscrowError, state::NativeEscrowState };
// Step 13: Pack trait is unused in this file, but originally imported for reference.
// use solana_program::program_pack::Pack;

// Step 14: Define the initialize handler function to set up escrow state and deposit tokens.
pub fn initialize(
    // Step 15: Accept the program ID under which this program is executing.
    program_id: &Pubkey,
    // Step 16: Accept the list of accounts passed to this instruction.
    accounts: &[AccountInfo],
    // Step 17: Accept the serialized parameters from the transaction instruction data.
    instruction_data: &[u8]
) -> ProgramResult {
    // ── Raw instruction packers (stack arrays, no heap) ──────

    // Step 18: Define an inline function to construct a system_instruction::create_account payload manually.
    fn pack_create_account(lamports: u64, space: u64, owner: &Pubkey) -> [u8; 52] {
        // Step 19: Allocate a fixed-size 52-byte buffer on the stack.
        let mut buf = [0u8; 52];
        // Step 20: Write the 4-byte instruction discriminator (0u32 for System CreateAccount).
        buf[0..4].copy_from_slice(&(0u32).to_le_bytes());
        // Step 21: Write the 8-byte u64 lamports funding amount.
        buf[4..12].copy_from_slice(&lamports.to_le_bytes());
        // Step 22: Write the 8-byte u64 data space size allocation.
        buf[12..20].copy_from_slice(&space.to_le_bytes());
        // Step 23: Write the 32-byte Pubkey of the program owner.
        buf[20..52].copy_from_slice(&owner.to_bytes());
        // Step 24: Return the fully packed 52-byte instruction payload.
        buf
    }

    // Step 25: Define an inline function to construct an spl_token::instruction::transfer_checked payload.
    fn pack_transfer_checked(amount: u64, decimals: u8) -> [u8; 10] {
        // Step 26: Allocate a fixed-size 10-byte buffer on the stack.
        let mut buf = [0u8; 10];
        // Step 27: Set the first byte to the instruction opcode (12 for TransferChecked).
        buf[0] = 12;
        // Step 28: Write the 8-byte u64 transfer amount.
        buf[1..9].copy_from_slice(&amount.to_le_bytes());
        // Step 29: Set the final byte to the mint's decimals count.
        buf[9] = decimals;
        // Step 30: Return the fully packed 10-byte instruction payload.
        buf
    }

    // Step 31: Log the execution of the initialization function.
    msg!("Native Escrow: Initialize zero copy");
    // Step 32: Ensure at least 7 accounts are passed to prevent index out of bounds.
    if accounts.len() < 7 {
        // Step 33: Return InvalidInstructionData if the account slice is too small.
        return Err(EscrowError::InvalidInstructionData.into());
    }

    // Direct pointer extraction – zero allocation, bypassing iterator overhead
    // Step 34: Reference the maker account (index 0) - creator and source of funds.
    let maker = &accounts[0];
    // Step 35: Reference the stablecoin mint account (index 1).
    let stablecoin_mint = &accounts[1];
    // Step 36: Reference the maker's token account (index 2).
    let maker_token_account = &accounts[2];
    // Step 37: Reference the Escrow PDA account (index 3) that holds state.
    let escrow_account = &accounts[3];
    // Step 38: Reference the Vault token account PDA (index 4) that holds deposited stablecoins.
    let escrow_vault = &accounts[4];
    // Step 39: Reference the System Program account (index 5) for creating the escrow account.
    let _system_program = &accounts[5];
    // Step 40: Reference the Token Program account (index 6) for executing token transfers.
    let token_program = &accounts[6];

    // === 1. Basic validation ===
    // Step 41: Ensure the maker signed the transaction to authorize creation/funding.
    if !maker.is_signer {
        // Step 42: Return InvalidAuthority if the maker is not a signer.
        return Err(EscrowError::InvalidAuthority.into());
    }

    // Step 43: Check if the escrow account is unallocated (empty, 0 data length).
    let is_fresh_account = escrow_account.data_len() == 0;

    // Step 44: If the account is already allocated, ensure it is owned by this program.
    if !is_fresh_account && *escrow_account.owner != *program_id {
        // Step 45: Return InvalidAccountOwner if owner doesn't match this program.
        return Err(EscrowError::InvalidAccountOwner.into());
    }

    // Strict Token-2022 Enforcement
    // Step 46: Ensure the provided token program matches Token-2022 ID.
    if token_program.key != &spl_token_2022::ID {
        // Step 47: Return InvalidTokenProgram if it does not match spl_token_2022::ID.
        return Err(EscrowError::InvalidTokenProgram.into());
    }

    // === 2. Exact Instruction Data Parsing ===
    // Step 48: Define the expected length of the serialized initialization parameters (8+2+32+32+32 = 106).
    const PARAMS_LEN: usize = 8 + 2 + 32 + 32 + 32;
    // Step 49: Ensure the instruction data slice matches the exact length.
    if instruction_data.len() != PARAMS_LEN {
        // Step 50: Return InvalidInstructionData if the parameter length is incorrect.
        return Err(EscrowError::InvalidInstructionData.into());
    }

    // Step 51: Parse the 8-byte u64 amount from instruction data.
    let amount = u64::from_le_bytes(
        instruction_data[0..8].try_into().map_err(|_| ProgramError::InvalidInstructionData)?
    );
    // Step 52: Parse the 2-byte u16 transfer fee in basis points from instruction data.
    let transfer_fee_bps = u16::from_le_bytes(
        instruction_data[8..10].try_into().map_err(|_| ProgramError::InvalidInstructionData)?
    );
    // Step 53: Parse the 32-byte Pubkey representing the designated taker.
    let taker = Pubkey::try_from(&instruction_data[10..42]).map_err(
        |_| ProgramError::InvalidInstructionData
    )?;
    // Step 54: Parse the 32-byte Merkle root of the allowlist.
    let allowlist_root = <[u8; 32]>
        ::try_from(&instruction_data[42..74])
        .map_err(|_| ProgramError::InvalidInstructionData)?;
    // Step 55: Parse the 32-byte Merkle root of the blacklist.
    let blacklist_root = <[u8; 32]>
        ::try_from(&instruction_data[74..106])
        .map_err(|_| ProgramError::InvalidInstructionData)?;

    // Step 56: Check that the transfer amount is non-zero and that fees do not exceed 100% (10000 BPS).
    if amount == 0 || transfer_fee_bps > 10000 {
        // Step 57: Return InvalidAmount if validation fails.
        return Err(EscrowError::InvalidAmount.into());
    }

    // === 3. PDA Validation ===
    // Step 58: Deterministically derive the Escrow PDA using seeds: "escrow", maker, taker, mint.
    let (expected_escrow_key, escrow_bump) = Pubkey::find_program_address(
        &[b"escrow", maker.key.as_ref(), taker.as_ref(), stablecoin_mint.key.as_ref()],
        program_id
    );
    // Step 59: Verify that the derived escrow key matches the passed escrow account public key.
    if expected_escrow_key != *escrow_account.key {
        // Step 60: Return InvalidEscrowPDA if the keys do not match.
        return Err(EscrowError::InvalidEscrowPDA.into());
    }

    // Step 61: Deterministically derive the Vault PDA using seeds: "vault", escrow_account.
    let (expected_vault_key, _) = Pubkey::find_program_address(
        &[b"vault", escrow_account.key.as_ref()],
        program_id
    );
    // Step 62: Verify that the derived vault key matches the passed vault account public key.
    if expected_vault_key != *escrow_vault.key {
        // Step 63: Return InvalidVaultPDA if the keys do not match.
        return Err(EscrowError::InvalidVaultPDA.into());
    }
    // Step 64: Verify that the vault PDA is marked writable in the transaction.
    if !escrow_vault.is_writable {
        // Step 65: Return InvalidAccountData if vault is not writable.
        return Err(EscrowError::InvalidAccountData.into());
    }

    // === 4. Raw CreateAccount CPI (replaces system_instruction::create_account) ===
    // Step 66: If the escrow account is fresh and has no data, create it using System Program.
    if is_fresh_account {
        // Step 67: Fetch the system rent parameters.
        let rent = Rent::get()?;
        // Step 68: Calculate the minimum lamports required for rent exemption of the EscrowState.
        let required_lamports = rent.minimum_balance(NativeEscrowState::LEN);

        // Step 69: Pack the CreateAccount instruction data using stack buffer.
        let payload = pack_create_account(
            required_lamports,
            NativeEscrowState::LEN as u64,
            program_id
        );

        // Step 70: Construct the System Program instruction layout manually.
        let create_ix = Instruction {
            program_id: solana_program::system_program::ID,
            accounts: vec![
                AccountMeta::new(*maker.key, true), // Maker is payer and signer
                AccountMeta::new(*escrow_account.key, true) // Escrow PDA is writable and signer (via seeds)
            ],
            data: payload.to_vec(),
        };

        // Step 71: Invoke the System Program signed with the Escrow PDA seeds.
        invoke_signed(
            &create_ix,
            &[maker.clone(), escrow_account.clone()],
            &[
                &[
                    b"escrow",
                    maker.key.as_ref(),
                    taker.as_ref(),
                    stablecoin_mint.key.as_ref(),
                    &[escrow_bump],
                ],
            ]
        )?;
    }

    // === 5. Alignment-safe Zero-Copy Write ===
    {
        // Step 72: Borrow a mutable reference to the escrow account's underlying data.
        let mut data = escrow_account.try_borrow_mut_data()?;
        // Step 73: Verify that the allocated data slice is large enough to fit NativeEscrowState.
        if data.len() < NativeEscrowState::LEN {
            // Step 74: Return InvalidAccountData if data length is too small.
            return Err(EscrowError::InvalidAccountData.into());
        }

        // Step 75: Cast the raw mutable slice to a mutable reference of NativeEscrowState using bytemuck.
        let escrow_state: &mut NativeEscrowState = try_from_bytes_mut(
            &mut data[..NativeEscrowState::LEN]
        ).map_err(|_| EscrowError::InvalidAccountData)?;

        // Step 76: Verify that the account has not already been initialized previously.
        if escrow_state.is_initialized == 1 {
            // Step 77: Return AlreadyInitialized if state is already active.
            return Err(EscrowError::AlreadyInitialized.into());
        }

        // Step 78: Perform alignment-safe copy from memory fields to state.
        *escrow_state = NativeEscrowState {
            maker: maker.key.to_bytes(),
            taker: taker.to_bytes(),
            stablecoin_mint: stablecoin_mint.key.to_bytes(),
            amount,
            transfer_fee_bps,
            allowlist_merkle_root: allowlist_root,
            blacklist_merkle_root: blacklist_root,
            bump: escrow_bump,
            is_initialized: 1,
            cancel_authority: maker.key.to_bytes(),
            _padding: [0; 4],
        };
    }

    // Step 79: Retrieve decimals count directly from the mint account data at offset 44.
    let decimals = {
        // Step 80: Borrow the mint account data slice.
        let mint_data = stablecoin_mint.try_borrow_data()?;
        // Step 81: Ensure the mint account has at least 45 bytes to read decimals.
        if mint_data.len() < 45 {
            // Step 82: Return InvalidMint if mint account layout is invalid.
            return Err(EscrowError::InvalidMint.into());
        }
        // Step 83: Extract the decimals byte at position 44 (standard SPL mint layout).
        mint_data[44]
    }; // Step 84: Borrow is implicitly dropped here.

    // Step 85: Pack the TransferChecked instruction data payload.
    let xfer_payload = pack_transfer_checked(amount, decimals);
    // Step 86: Construct the manual token transfer instruction targeting the Token program.
    let transfer_ix = Instruction {
        program_id: *token_program.key,
        accounts: vec![
            AccountMeta::new(*maker_token_account.key, false), // Source token account (maker)
            AccountMeta::new_readonly(*stablecoin_mint.key, false), // Mint
            AccountMeta::new(*escrow_vault.key, false), // Destination token account (Vault)
            AccountMeta::new_readonly(*maker.key, true) // Maker authority signature
        ],
        data: xfer_payload.to_vec(),
    };

    // Step 87: Invoke the token transfer CPI using the prepared parameters.
    invoke(
        &transfer_ix,
        &[
            maker_token_account.clone(),
            stablecoin_mint.clone(),
            escrow_vault.clone(),
            maker.clone(),
            token_program.clone(),
        ]
    )?;

    // Step 88: Log successful initialization.
    msg!("NativeEscrow: Initialized (raw CPI)");
    // Step 89: Return success.
    Ok(())
}
