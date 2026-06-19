use anchor_lang::prelude::*;
use anchor_spl::token_interface::{
    transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked,
};
use crate::errors::EscrowError;
use crate::state::AnchorStablecoinEscrow;

#[derive(Accounts)]
#[instruction(
    amount: u64,
    transfer_fee_bps: u16,
    allowlist_merkle_root: [u8; 32],
    blacklist_merkle_root: [u8; 32],
    taker: Pubkey,                    // ← needed for PDA seeds and storage
    cancel_authority: Pubkey,
)]
pub struct InitializeEscrow<'info> {
    #[account(mut)]
    pub maker: Signer<'info>,

    /// The stablecoin mint (Token‑2022 compatible)
    pub stablecoin_mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        token::mint = stablecoin_mint,
        token::authority = maker,
    )]
    pub maker_token_account: InterfaceAccount<'info, TokenAccount>,

    /// Escrow state PDA — stores all parameters
    #[account(
        init,
        payer = maker,
        space = AnchorStablecoinEscrow::INIT_SPACE,
        seeds = [
            b"escrow",
            maker.key().as_ref(),
            taker.as_ref(),
            stablecoin_mint.key().as_ref(),
        ],
        bump,
    )]
    pub escrow_account: Account<'info, AnchorStablecoinEscrow>,

    /// Vault token account — holds the locked tokens, authority = escrow PDA
    #[account(
        init,
        payer = maker,
        token::mint = stablecoin_mint,
        token::authority = escrow_account,          // the PDA
        token::token_program = token_program,
    )]
    pub escrow_vault: InterfaceAccount<'info, TokenAccount>,

    pub system_program: Program<'info, System>,

    pub token_program: Interface<'info, TokenInterface>,
}

pub fn handler(
    ctx: Context<InitializeEscrow>,
    amount: u64,
    transfer_fee_bps: u16,
    allowlist_merkle_root: [u8; 32],
    blacklist_merkle_root: [u8; 32],
    taker: Pubkey,    // ← must be here too, matches the #[instruction] args
    cancel_authority: Pubkey,
) -> Result<()> {
    let escrow = &mut ctx.accounts.escrow_account;
    escrow.cancel_authority = cancel_authority;
    // ── Guard clauses ────────────────────────────────────────
    require!(amount > 0, EscrowError::InvalidAmount);
    require!(transfer_fee_bps <= 10_000, EscrowError::InvalidFeeBps);
    require!(
        ctx.accounts.maker_token_account.amount >= amount,
        EscrowError::InsufficientBalance
    );

    // ── Store escrow parameters ──────────────────────────────
    escrow.maker = *ctx.accounts.maker.key;
    escrow.taker = taker;
    escrow.stablecoin_mint = *ctx.accounts.stablecoin_mint.to_account_info().key;
    escrow.amount = amount;
    escrow.transfer_fee_bps = transfer_fee_bps;
    escrow.allowlist_merkle_root = allowlist_merkle_root;
    escrow.blacklist_merkle_root = blacklist_merkle_root;
    escrow.bump = ctx.bumps.escrow_account;
    escrow.is_initialized = true;

    // ── Atomic transfer from maker to vault ─────────────────
    let cpi_accounts = TransferChecked {
        from: ctx.accounts.maker_token_account.to_account_info(),
        mint: ctx.accounts.stablecoin_mint.to_account_info(),
        to: ctx.accounts.escrow_vault.to_account_info(),     // vault, not escrow_account
        authority: ctx.accounts.maker.to_account_info(),
    };

    let cpi_ctx = CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        cpi_accounts,
    );
    transfer_checked(cpi_ctx, amount, ctx.accounts.stablecoin_mint.decimals)?;

    emit!(EscrowInitialized {
        maker: escrow.maker,
        taker: escrow.taker,
        amount,
        bump: escrow.bump,
        cancel_authority: escrow.cancel_authority,
    });

    Ok(())
}

#[event]
pub struct EscrowInitialized {
    pub maker: Pubkey,
    pub taker: Pubkey,
    pub amount: u64,
    pub bump: u8,
    pub cancel_authority: Pubkey,
}