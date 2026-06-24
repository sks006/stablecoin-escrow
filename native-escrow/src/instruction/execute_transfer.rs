// Step 1: Import AccountInfo representing the metadata and pointers of Solana accounts.
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
    // Step 10: Reference to the taker (buyer) account who must sign the transaction.
    taker:               &'a AccountInfo<'info>,  // Account index 0
    // Step 11: Reference to the Escrow PDA account containing configuration state.
    escrow_account:      &'a AccountInfo<'info>,  // Account index 1
    // Step 12: Reference to the Escrow Vault token account PDA holding deposited tokens.
    escrow_vault:        &'a AccountInfo<'info>,  // Account index 2
    // Step 13: Reference to the Taker's token account destination for settlement.
    taker_token_account: &'a AccountInfo<'info>,  // Account index 3
    // Step 14: Reference to the Maker's token account destination for fee distribution.
    maker_token_account: &'a AccountInfo<'info>,  // Account index 4
    // Step 15: Reference to the Maker's system account to receive reclaimed rent lamports.
    maker_ref:           &'a AccountInfo<'info>,  // Account index 5
    // Step 16: Reference to the stablecoin Mint account.
    stablecoin_mint:     &'a AccountInfo<'info>,  // Account index 6
    // Step 17: Reference to the Token Program account.
    token_program:       &'a AccountInfo<'info>,  // Account index 7
}

// Step 18: Implement extract method for Accts struct to fetch and validate accounts.
impl<'a, 'info> Accts<'a, 'info> {
    // Step 19: Inline extraction function with compiler optimization hint.
    #[inline(always)]
    fn extract(accounts: &'a [AccountInfo<'info>]) -> Result<Self, EscrowError> {
        // Step 20: Check that at least 8 accounts are passed to prevent index out of bounds.
        if accounts.len() < 8 {
            // Step 21: Return InvalidInstructionData if the account slice is too small.
            return Err(EscrowError::InvalidInstructionData);
        }
        // Step 22: Construct and return the Accts structure with extracted references.
        Ok(Self {
            taker:               &accounts[0],
            escrow_account:      &accounts[1],
            escrow_vault:        &accounts[2],
            taker_token_account: &accounts[3],
            maker_token_account: &accounts[4],
            maker_ref:           &accounts[5],
            stablecoin_mint:     &accounts[6],
            token_program:       &accounts[7],
        })
    }
}

// Step 23: Define helper to manually build a TransferChecked instruction.
#[inline(always)]
fn xfer_checked(
    prog: &Pubkey, src: &Pubkey, mint: &Pubkey, dst: &Pubkey, auth: &Pubkey,
    amount: u64, decimals: u8,
) -> Instruction {
    // Step 24: Create a 10-byte instruction payload.
    let mut data = [0u8; 10];
    // Step 25: Opcode 12 represents TransferChecked in SPL Token program.
    data[0] = 12;
    // Step 26: Copy the 8-byte transfer amount.
    data[1..9].copy_from_slice(&amount.to_le_bytes());
    // Step 27: Set the last byte to decimals.
    data[9] = decimals;
    // Step 28: Construct and return the Instruction.
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

// Step 29: Define helper to manually build a CloseAccount instruction.
#[inline(always)]
fn close_acct(
    prog: &Pubkey, account: &Pubkey, dest: &Pubkey, auth: &Pubkey,
) -> Instruction {
    // Step 30: Construct and return Instruction with opcode 9 (CloseAccount) and account metadata.
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

// Step 31: Define execute_transfer function to perform token settlement and close the escrow accounts.
pub fn execute_transfer(
    // Step 32: Accept the program ID under which this program is executing.
    program_id: &Pubkey,
    // Step 33: Accept the list of accounts passed to this instruction.
    accounts: &[AccountInfo],
    // Step 34: Accept unused instruction parameters.
    _instruction_data: &[u8],
) -> ProgramResult {
    // ── L1: Single bounds check, direct extraction ──────────────────
    // Step 35: Extract account references from slice.
    let a = Accts::extract(accounts)?;

    // ── 1. Guards ───────────────────────────────────────────────────
    // Step 36: Verify that the taker has signed the transaction.
    if !a.taker.is_signer {
        // Step 37: Return InvalidAuthority if signer check fails.
        return Err(EscrowError::InvalidAuthority.into());
    }
    // Step 38: Ensure the escrow state account is owned by this program.
    if *a.escrow_account.owner != *program_id {
        // Step 39: Return InvalidAccountOwner if owner doesn't match program_id.
        return Err(EscrowError::InvalidAccountOwner.into());
    }
    // Step 40: Ensure the token program ID matches Token-2022.
    if a.token_program.key != &spl_token_2022::ID {
        // Step 41: Return InvalidTokenProgram if it does not match Token-2022.
        return Err(EscrowError::InvalidTokenProgram.into());
    }

    // ── 2. Single-borrow state extraction ───────────────────────────
    // Step 42: Scope borrow to read configuration state details.
    let (amount, fee_bps, bump, maker_key, taker_key, mint_key) = {
        // Step 43: Borrow read-only access to escrow account data.
        let d = a.escrow_account.try_borrow_data()?;
        // Step 44: Validate that the account size is large enough to contain the state struct.
        if d.len() < NativeEscrowState::LEN {
            // Step 45: Return InvalidAccountData if layout size is too small.
            return Err(EscrowError::InvalidAccountData.into());
        }
        // Step 46: Cast raw byte slice to NativeEscrowState reference.
        let s: &NativeEscrowState = try_from_bytes(&d[..NativeEscrowState::LEN])
            .map_err(|_| EscrowError::InvalidAccountData)?;
        // Step 47: Ensure the escrow is initialized.
        if s.is_initialized == 0 {
            // Step 48: Return NotInitialized if the flag is 0.
            return Err(EscrowError::NotInitialized.into());
        }
        // Step 49: Validate that the transaction's taker matches the designated taker.
        if s.taker != a.taker.key.to_bytes() {
            // Step 50: Return InvalidTaker if mismatch is found.
            return Err(EscrowError::InvalidTaker.into());
        }
        // Step 51: Return tuple of amount, transfer_fee_bps, bump, maker, taker, stablecoin_mint.
        (s.amount, s.transfer_fee_bps, s.bump,
         s.maker, s.taker, s.stablecoin_mint)
    }; // Step 52: Borrow is dropped here.

    // ── L2: PDA verification — one hash ─────────────────────────────
    // Step 53: Define the PDA signing seeds for the Escrow account.
    let seeds: &[&[u8]] = &[
        b"escrow",
        maker_key.as_ref(),
        taker_key.as_ref(),
        mint_key.as_ref(),
        &[bump],
    ];
    // Step 54: Compute the expected Escrow PDA using create_program_address (saves gas).
    let expected = Pubkey::create_program_address(seeds, program_id)
        .map_err(|_| EscrowError::InvalidEscrowPDA)?;
    // Step 55: Validate that the derived PDA matches the passed escrow account public key.
    if expected != *a.escrow_account.key {
        // Step 56: Return InvalidEscrowPDA if mismatch.
        return Err(EscrowError::InvalidEscrowPDA.into());
    }
    // Step 57: Construct PDA signing helper array.
    let signer = &[&seeds[..]];

    // ── 3. Fee calculation ──────────────────────────────────────────
    // Step 58: Calculate the fee amount using a ceil-division algorithm.
    let fee_amount = if fee_bps > 0 && amount > 0 {
        // Step 59: Perform calculation using u128 to prevent overflow during intermediate step.
        let f = (amount as u128)
            .checked_mul(fee_bps as u128)
            .and_then(|v| v.checked_add(9_999))
            .and_then(|v| v.checked_div(10_000))
            .ok_or(EscrowError::FeeCalculationError)?;
        // Step 60: Cap the fee amount to the total amount.
        u64::try_from(f).unwrap().min(amount)
    } else {
        // Step 61: Set fee to zero if BPS or amount is zero.
        0
    };
    // Step 62: Subtract fee from total amount to determine net transfer amount.
    let net = amount.checked_sub(fee_amount).ok_or(EscrowError::Overflow)?;
    // Step 63: Validate that the net transfer amount is greater than zero.
    if net == 0 {
        // Step 64: Return error if fee consumes the entire amount.
        return Err(EscrowError::InvalidAmountAfterFee.into());
    }

    // ── L4: Raw decimals read — byte offset 44 ──────────────────────
    // Step 65: Read decimals directly from mint account without deserialization.
    let decimals = {
        // Step 66: Borrow read-only mint account data.
        let md = a.stablecoin_mint.try_borrow_data()?;
        // Step 67: Validate that mint data contains at least 45 bytes.
        if md.len() < 45 { return Err(EscrowError::InvalidMint.into()); }
        // Step 68: Extract the decimals byte at offset 44.
        md[44]
    }; // Step 69: Mint borrow dropped here.

    // ── L3: Smart path selection based on fee structure ─────────────
    //
    // ZERO-FEE PATH: fee_amount == 0 means net == amount == vault_balance.
    //   → Transfer entire vault to taker (1 CPI)
    //   → Skip vault balance read entirely
    //   → Skip dust sweep CPI entirely
    //   → Saves ~10,700 CUs (vault read + instruction build + CPI)
    //
    // FEE PATH: fee_amount > 0 means residual dust in vault after net transfer.
    //   → Transfer net to taker (CPI #1)
    //   → Read vault, sweep remainder to maker (CPI #2)

    // CPI #1: Transfer net amount to taker
    // Step 70: Execute signed TransferChecked CPI to transfer net amount to taker_token_account.
    invoke_signed(
        &xfer_checked(
            a.token_program.key, a.escrow_vault.key, a.stablecoin_mint.key,
            a.taker_token_account.key, a.escrow_account.key, net, decimals,
        ),
        &[
            a.escrow_vault.clone(),
            a.stablecoin_mint.clone(),
            a.taker_token_account.clone(),
            a.escrow_account.clone(),
            a.token_program.clone(),
        ],
        signer,
    )?;

    // L3: Conditional dust sweep — only when fee creates residual
    // Step 71: Sweep the remaining fee/dust tokens to maker only if fees were enabled.
    if fee_amount > 0 {
        // Step 72: Borrow vault data to read remaining balance.
        let vault_data = a.escrow_vault.try_borrow_data()?;
        // Step 73: Cast raw bytes to TokenAccountBase reference.
        let vault_base: &TokenAccountBase = try_from_bytes(
            &vault_data[..std::mem::size_of::<TokenAccountBase>()]
        ).map_err(|_| EscrowError::InvalidAccountData)?;
        // Step 74: Parse remaining balance from u64 byte array.
        let remaining = u64::from_le_bytes(vault_base.amount);
        // Step 75: Manually drop vault data borrow.
        drop(vault_data);

        // Step 76: If there are remaining dust tokens, transfer them to the maker.
        if remaining > 0 {
            // Step 77: Execute signed TransferChecked CPI to transfer remaining balance to maker_token_account.
            invoke_signed(
                &xfer_checked(
                    a.token_program.key, a.escrow_vault.key, a.stablecoin_mint.key,
                    a.maker_token_account.key, a.escrow_account.key, remaining, decimals,
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
    }

    // ── Close vault (rent → maker_ref) ──────────────────────────────
    // Step 78: Execute signed CloseAccount CPI to close vault account and refund lamports to maker_ref.
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

    // ── Drain escrow lamports + zero data ───────────────────────────
    {
        // Step 79: Borrow mutable reference to escrow account lamports.
        let mut src = a.escrow_account.lamports.borrow_mut();
        // Step 80: Borrow mutable reference to maker_ref lamports.
        let mut dst = a.maker_ref.lamports.borrow_mut();
        // Step 81: Add escrow lamports to maker_ref balance safely.
        **dst = dst.checked_add(**src).unwrap();
        // Step 82: Set escrow lamports to zero.
        **src = 0;
    }
    {
        // Step 83: Borrow mutable reference to escrow account data.
        let mut data = a.escrow_account.try_borrow_mut_data()?;
        // Step 84: Zero-out all bytes in the escrow account to prevent stale data reuse.
        data.fill(0);
    }

    // Step 85: Return success.
    Ok(())
}