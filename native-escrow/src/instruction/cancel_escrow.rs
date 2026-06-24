// Step 1: Import AccountInfo struct representing metadata and pointers for Solana accounts.
use solana_program::account_info::AccountInfo;
// Step 2: Import ProgramResult representing the success/failure state of execution.
use solana_program::entrypoint::ProgramResult;
// Step 3: Import Instruction and AccountMeta structures to construct custom instructions for CPIs.
use solana_program::instruction::{AccountMeta, Instruction};
// Step 4: Import invoke_signed to execute Cross-Program Invocations utilizing PDA signer seeds.
use solana_program::program::invoke_signed;
// Step 5: Import Pubkey type representing 32-byte public keys of accounts/programs.
use solana_program::pubkey::Pubkey;
// Step 6: Import try_from_bytes helper from bytemuck for zero-copy read-only data casting.
use bytemuck::try_from_bytes;
// Step 7: Import NativeEscrowState and TokenAccountBase structs defining memory layouts of accounts.
use crate::state::{NativeEscrowState, TokenAccountBase};
// Step 8: Import custom EscrowError enum representing program errors.
use crate::errors::EscrowError;

// Step 9: Define Accts container struct to reference required instruction accounts via index.
struct Accts<'a, 'info> {
    // Step 10: Reference to cancel authority (maker) account that authorizes escrow cancellation.
    cancel_authority:    &'a AccountInfo<'info>,  // Account index 0
    // Step 11: Reference to the Escrow PDA account containing configuration state.
    escrow_account:      &'a AccountInfo<'info>,  // Account index 1
    // Step 12: Reference to the Escrow Vault token account PDA holding deposited tokens.
    escrow_vault:        &'a AccountInfo<'info>,  // Account index 2
    // Step 13: Reference to the Maker's token account destination for refund.
    maker_token_account: &'a AccountInfo<'info>,  // Account index 3
    // Step 14: Reference to the Maker's system account to receive reclaimed rent lamports.
    maker_ref:           &'a AccountInfo<'info>,  // Account index 4
    // Step 15: Reference to the stablecoin Mint account.
    stablecoin_mint:     &'a AccountInfo<'info>,  // Account index 5
    // Step 16: Reference to the Token Program account.
    token_program:       &'a AccountInfo<'info>,  // Account index 6
}

// Step 17: Implement extract method for Accts struct to fetch and validate accounts.
impl<'a, 'info> Accts<'a, 'info> {
    // Step 18: Inline extraction function with compiler optimization hint.
    #[inline(always)]
    fn extract(accounts: &'a [AccountInfo<'info>]) -> Result<Self, EscrowError> {
        // Step 19: Check that at least 7 accounts are passed to prevent index out of bounds.
        if accounts.len() < 7 {
            // Step 20: Return InvalidInstructionData if the account slice is too small.
            return Err(EscrowError::InvalidInstructionData);
        }
        // Step 21: Construct and return the Accts structure with extracted references.
        Ok(Self {
            cancel_authority:    &accounts[0],
            escrow_account:      &accounts[1],
            escrow_vault:        &accounts[2],
            maker_token_account: &accounts[3],
            maker_ref:           &accounts[4],
            stablecoin_mint:     &accounts[5],
            token_program:       &accounts[6],
        })
    }
}

// Step 22: Define helper to manually build a TransferChecked instruction.
#[inline(always)]
fn xfer_checked(
    prog: &Pubkey, src: &Pubkey, mint: &Pubkey, dst: &Pubkey, auth: &Pubkey,
    amount: u64, decimals: u8,
) -> Instruction {
    // Step 23: Create a 10-byte instruction payload.
    let mut data = [0u8; 10];
    // Step 24: Opcode 12 represents TransferChecked in SPL Token program.
    data[0] = 12;
    // Step 25: Copy the 8-byte transfer amount.
    data[1..9].copy_from_slice(&amount.to_le_bytes());
    // Step 26: Set the last byte to decimals.
    data[9] = decimals;
    // Step 27: Construct and return the Instruction.
    Instruction {
        program_id: *prog,
        accounts: vec![
            AccountMeta::new(*src, false),
            AccountMeta::new_readonly(*mint, false),
            AccountMeta::new(*dst, false),
            AccountMeta::new_readonly(*auth, true),
        ],
        data: data.to_vec(),
    }
}

// Step 28: Define helper to manually build a CloseAccount instruction.
#[inline(always)]
fn close_acct(
    prog: &Pubkey, account: &Pubkey, dest: &Pubkey, auth: &Pubkey,
) -> Instruction {
    // Step 29: Construct and return Instruction with opcode 9 (CloseAccount) and account metadata.
    Instruction {
        program_id: *prog,
        accounts: vec![
            AccountMeta::new(*account, false),
            AccountMeta::new(*dest, false),
            AccountMeta::new_readonly(*auth, true),
        ],
        data: vec![9],
    }
}

// Step 30: Define cancel_escrow function to close vault, return tokens, and refund rent.
pub fn cancel_escrow(
    // Step 31: Accept the program ID under which this program is executing.
    program_id: &Pubkey,
    // Step 32: Accept the list of accounts passed to this instruction.
    accounts: &[AccountInfo],
    // Step 33: Accept unused instruction parameters.
    _instruction_data: &[u8],
) -> ProgramResult {
    // ── L1: Direct slice extraction ──────────────────────────────
    // Step 34: Extract account references from slice.
    let a = Accts::extract(accounts)?;

    // ── 1. Guards ─────────────────────────────────────────────────
    // Step 35: Verify that the cancel authority has signed the transaction.
    if !a.cancel_authority.is_signer {
        // Step 36: Return InvalidAuthority if signer check fails.
        return Err(EscrowError::InvalidAuthority.into());
    }
    // Step 37: Ensure the escrow state account is owned by this program.
    if *a.escrow_account.owner != *program_id {
        // Step 38: Return InvalidAccountOwner if owner doesn't match program_id.
        return Err(EscrowError::InvalidAccountOwner.into());
    }
    // Step 39: Ensure the token program ID matches Token-2022.
    if a.token_program.key != &spl_token_2022::ID {
        // Step 40: Return InvalidTokenProgram if it does not match Token-2022.
        return Err(EscrowError::InvalidTokenProgram.into());
    }

    // ── 2. Single‑borrow state extraction ────────────────────────
    // Step 41: Scope borrow to read configuration state details.
    let (bump, maker_key, taker_key, mint_key) = {
        // Step 42: Borrow read-only access to escrow account data.
        let d = a.escrow_account.try_borrow_data()?;
        // Step 43: Validate that the account size is large enough to contain the state struct.
        if d.len() < NativeEscrowState::LEN {
            // Step 44: Return InvalidAccountData if layout size is too small.
            return Err(EscrowError::InvalidAccountData.into());
        }
        // Step 45: Cast raw byte slice to NativeEscrowState reference.
        let s: &NativeEscrowState = try_from_bytes(&d[..NativeEscrowState::LEN])
            .map_err(|_| EscrowError::InvalidAccountData)?;
        // Step 46: Ensure the escrow is initialized.
        if s.is_initialized == 0 {
            // Step 47: Return NotInitialized if the flag is 0.
            return Err(EscrowError::NotInitialized.into());
        }
        // 🛡️ Cancel authority check using byte arrays
        // Step 48: Validate that the transaction's cancel_authority matches the stored authority.
        if s.cancel_authority != a.cancel_authority.key.to_bytes() {
            // Step 49: Return UnauthorizedCanceller if mismatch is found.
            return Err(EscrowError::UnauthorizedCanceller.into());
        }
        // Step 50: Return tuple of bump, maker, taker, stablecoin_mint.
        (s.bump, s.maker, s.taker, s.stablecoin_mint)
    }; // Step 51: Borrow is dropped here.

    // ── L2: PDA seeds ─────────────────────────────────────────────
    // Step 52: Define the PDA signing seeds for the Escrow account.
    let seeds: &[&[u8]] = &[
        b"escrow",
        maker_key.as_ref(),
        taker_key.as_ref(),
        mint_key.as_ref(),
        &[bump],
    ];
    // Step 53: Construct PDA signing helper array.
    let signer = &[&seeds[..]];

    // ── L4: Raw decimals read at offset 44 ─────────────────────────
    // Step 54: Read decimals directly from mint account without deserialization.
    let decimals = {
        // Step 55: Borrow read-only mint account data.
        let md = a.stablecoin_mint.try_borrow_data()?;
        // Step 56: Validate that mint data contains at least 45 bytes.
        if md.len() < 45 { return Err(EscrowError::InvalidMint.into()); }
        // Step 57: Extract the decimals byte at offset 44.
        md[44]
    }; // Step 58: Mint borrow dropped here.

    // ── L5: Sweep entire vault balance to maker (raw TransferChecked) ──
    // Step 59: Read the current token balance of the Vault account.
    let vault_data = a.escrow_vault.try_borrow_data()?;
    // Step 60: Cast vault data to TokenAccountBase structure.
    let vault_base: &TokenAccountBase = try_from_bytes(
        &vault_data[..std::mem::size_of::<TokenAccountBase>()]
    ).map_err(|_| EscrowError::InvalidAccountData)?;
    // Step 61: Parse the 8-byte amount to get vault balance.
    let vault_balance = u64::from_le_bytes(vault_base.amount);
    // Step 62: Manually drop vault data borrow.
    drop(vault_data);

    // Step 63: If the vault contains tokens, sweep them back to the maker.
    if vault_balance > 0 {
        // Step 64: Execute signed TransferChecked CPI to transfer vault_balance back to maker.
        invoke_signed(
            &xfer_checked(
                a.token_program.key, a.escrow_vault.key, a.stablecoin_mint.key,
                a.maker_token_account.key, a.escrow_account.key, vault_balance, decimals,
            ),
            &[
                a.escrow_vault.clone(),
                a.stablecoin_mint.clone(),
                a.maker_token_account.clone(),
                a.escrow_account.clone(),
                a.token_program.clone(),
            ],
            signer,
        )?;
    }

    // ── L3: Close vault (rent refund to maker_ref) ─────────────────
    // Step 65: Execute signed CloseAccount CPI to close vault account and refund lamports.
    invoke_signed(
        &close_acct(
            a.token_program.key, a.escrow_vault.key,
            a.maker_ref.key, a.escrow_account.key,
        ),
        &[
            a.escrow_vault.clone(),
            a.maker_ref.clone(),
            a.escrow_account.clone(),
            a.token_program.clone(),
        ],
        signer,
    )?;

    // ── L6: Drain escrow lamports + zero data ──────────────────────
    {
        // Step 66: Borrow mutable reference to escrow account lamports.
        let mut src = a.escrow_account.lamports.borrow_mut();
        // Step 67: Borrow mutable reference to maker_ref lamports.
        let mut dst = a.maker_ref.lamports.borrow_mut();
        // Step 68: Add escrow lamports to maker_ref balance safely.
        **dst = dst.checked_add(**src).unwrap();
        // Step 69: Set escrow lamports to zero.
        **src = 0;
    }
    {
        // Step 70: Borrow mutable reference to escrow account data.
        let mut data = a.escrow_account.try_borrow_mut_data()?;
        // Step 71: Zero-out all bytes in the escrow account to prevent stale data reuse.
        data.fill(0);
    }

    // Step 72: Return success.
    Ok(())
}