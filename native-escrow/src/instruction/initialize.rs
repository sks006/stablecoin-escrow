use solana_program::{
    account_info::{ next_account_info, AccountInfo },
    entrypoint::ProgramResult,
    msg,
    program::invoke,
    program_error::ProgramError,
    pubkey::Pubkey,
};
use crate::{
    error::EscrowError,
    instruction::InitializeEscrowInstruction,
    state::{ EscrowAccount, EscrowAccountData },
};

use anchor_spl::token_interface::{ self, TokenInterface, TransferChecked };

pub fn initialize(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8]
) -> ProgramResult {
    msg!("Native Escrow: Initialize zero copy");
    let account_iter = &mut accounts.iter();

    let maker = next_account_info(account_iter)?;
    let stablecoin_mint = next_account_info(account_iter)?;
    let maker_token_account = next_account_info(account_iter)?;
    let escrow_account = next_account_info(account_iter)?;
    let escrow_vault = next_account_info(account_iter)?;
    let system_program = next_account_info(account_iter)?;
    let token_program = next_account_info(account_iter)?;

    // === Basic validation ===

 if !maker.is_signer {
        return Err(EscrowError::InvalidAuthority.into());
    }

    // === Dynamic Gatekeeper (fixes Trap 1) ===
    let is_fresh_account = escrow_account.data_len() == 0;

    if !is_fresh_account && *escrow_account.owner != *program_id {
        return Err(EscrowError::InvalidAccountOwner.into());
    }

    // Token program check remains
    if token_program.key != &spl_token::ID && token_program.key != &spl_token_2022::ID {
        return Err(EscrowError::InvalidTokenProgram.into());
    }

    // === 2. Exact Instruction Data Parsing ===

    const PARAMS_LEN: usize = 8 + 2 + 32 + 32 + 32; // amount + fee + taker + allow + blacklist
    if instruction_data.len() != PARAMS_LEN {
        return Err(EscrowError::InvalidInstructionData.into());
    }

    let amount = u64::from_le_bytes(
        instruction_data[0..8].try_into().map_err(|_| ProgramError::InvalidInstructionData)?
    );
    let transfer_fee_bps = u16::from_le_bytes(
        instruction_data[8..10].try_into().map_err(|_| ProgramError::InvalidInstructionData)?
    );
    let taker = Pubkey::try_from(&instruction_data[10..42]).map_err(
        |_| ProgramError::InvalidInstructionData
    )?;
    let allowlist_root = <[u8; 32]>
        ::try_from(&instruction_data[42..74])
        .map_err(|_| ProgramError::InvalidInstructionData)?;
    let blacklist_root = <[u8; 32]>
        ::try_from(&instruction_data[74..106])
        .map_err(|_| ProgramError::InvalidInstructionData)?;

    if amount == 0 || transfer_fee_bps > 10000 {
        return Err(EscrowError::InvalidAmount.into());
    }

    // === 3. PDA Validation (Critical) ===

    let (expected_escrow_key, escrow_bump) = Pubkey::find_program_address(
        &[b"escrow", maker.key.as_ref(), taker.as_ref(), stablecoin_mint.key.as_ref()],
        program_id
    );
    if expected_escrow_key != *escrow_account.key {
        return Err(EscrowError::InvalidEscrowPDA.into());
    }

    // === 4. Vault PDA Verification ===

    let (expected_vault_key, _) = Pubkey::find_program_address(
        &[b"vault", escrow_account.key.as_ref()],
        program_id
    );
    if expected_vault_key != *escrow_vault.key {
        return Err(EscrowError::InvalidVaultPDA.into());
    }
    if !escrow_vault.is_writable {
        return Err(EscrowError::InvalidAccountData.into());
    }

    // === 4.5. System Program CPI: Create rent-exempt escrow state account ===
    
    if is_fresh_account {
        let rent = solana_program::rent::Rent::get()?;
        let required_lamports = rent.minimum_balance(NativeEscrowState::LEN);

        let create_ix = solana_program::system_instruction::create_account(
            maker.key,
            escrow_account.key,
            required_lamports,
            NativeEscrowState::LEN as u64,
            program_id
        );

        invoke_signed(
            &create_ix,
            &[
                maker.clone(),
                escrow_account.clone(),
                system_program.clone(),
                
            ],
            &[&[
                b"escrow",
                maker.key.as_ref(),
                taker.as_ref(),
                stablecoin_mint.key.as_ref(),
                &[escrow_bump],
            ]]

        )?;
    }
    // === 5. Alignment-safe Zero-Copy Write ===

    {
        let mut data = escrow_account.try_borrow_mut_data()?;
        if data.len() < NativeEscrowState::LEN {
            return Err(EscrowError::InvalidAccountData.into());
        }

        let escrow_state: &mut NativeEscrowState = try_from_bytes_mut(
            &mut data[..NativeEscrowState::LEN]
        ).map_err(|_| EscrowError::InvalidAccountData)?;

        if escrow_state.is_initialized == 1 {
            return Err(EscrowError::AlreadyInitialized.into());
        }

        *escrow_state = NativeEscrowState {
            maker: *maker.key,
            taker,
            stablecoin_mint: *stablecoin_mint.key,
            amount,
            transfer_fee_bps,
            allowlist_merkle_root: allowlist_root,
            blacklist_merkle_root: blacklist_root,
            bump: escrow_bump,
            is_initialized: 1,
            cancel_authority: *maker.key,
            _padding: [0; 4],
        };
    }

    // === 6. Real Decimals + Token Transfer ===

    let mint_data = stablecoin_mint.try_borrow_data()?;
    let mint = spl_token::state::Mint::unpack(&mint_data).map_err(|_| EscrowError::InvalidMint)?;

    let transfer_ix = token_interface::transfer_checked(
        token_program.key,
        maker_token_account.key,
        stablecoin_mint.key,
        escrow_vault.key,
        maker.key,
        &[],
        amount,
        mint.decimals
    )?;

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

    // Optional: Structured event
    msg!("escrow_initialized|{:?}|{:?}|{}", maker.key, taker, amount);

    msg!("NativeEscrow: Initialized successfully (zero-copy + all validations)");
    Ok(())
}
