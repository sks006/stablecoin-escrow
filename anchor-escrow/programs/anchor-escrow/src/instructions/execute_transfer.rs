// 'The Taker': The human who is receiving the funds (must be a Signer to authorize the receipt, or at least a mutable account depending on your specific flow).

// 'The Maker': The original creator. We need their account reference so we know who to refund the rent SOL to.

// 'The Escrow State (AnchorStablecoinEscrow)': We must read the data (amount, fee) and then destroy it.

// 'The Escrow Vault (TokenAccount)': The PDA-owned token account holding the funds. We must drain it and destroy it.

// 'The Taker's Token Account': The destination for the funds.

// 'The Mint & Token Program': Required for the CPI.

use anchor_lang::prelude::*;
use anchor_spl::token_interface::{
    transfer_checked,
    close_account,
    Mint,
    TokenAccount,
    TokenInterface,
    TransferChecked,
    CloseAccount,
};
use crate::errors::EscrowError;
use crate::state::AnchorStablecoinEscrow;

#[derive(Accounts)]
pub struct ExecuteTransfer<'info> {
      /// The taker must sign to claim the escrow
    pub taker: Signer<'info>,

    /// Taker's token account – will receive net tokens
    #[account(
        mut,
        token::mint = stablecoin_mint,
        token::authority = taker,
    )]
    pub taker_token_account: InterfaceAccount<'info, TokenAccount>,

    /// Maker's token account – will receive the fee
    #[account(
        mut,
        token::mint = stablecoin_mint,
        token::authority = maker_ref,
    )]
    pub maker_token_account: InterfaceAccount<'info, TokenAccount>,

    /// Escrow state PDA – validated and then destroyed
    #[account(
        mut,
        seeds = [
            b"escrow",
            maker_ref.key().as_ref(),
            taker.key().as_ref(),
            stablecoin_mint.key().as_ref(),
        ],
        bump = escrow_account.bump,
        constraint = escrow_account.taker == *taker.key @ EscrowError::UnauthorizedTaker,
        constraint = escrow_account.is_initialized @ EscrowError::NotInitialized,
        close = maker_ref,
    )]
    pub escrow_account: Account<'info, AnchorStablecoinEscrow>,

    /// Maker receives rent SOL refunds
    /// CHECK: only used as rent destination
    #[account(mut)]
    pub maker_ref: UncheckedAccount<'info>,

    /// Vault token account – holds locked tokens, will be drained and closed
    #[account(
        mut,
        token::mint = stablecoin_mint,
        token::authority = escrow_account,
    )]
    pub escrow_vault: InterfaceAccount<'info, TokenAccount>,

    pub stablecoin_mint: InterfaceAccount<'info, Mint>,

    /// Polymorphic token program (Token‑2022 or Token)
    pub token_program: Interface<'info, TokenInterface>,


}

pub fn handler(ctx: Context<ExecuteTransfer>) -> Result<()> {
    let escrow = &ctx.accounts.escrow_account;

    // === Fee calculation ===
    let amount = escrow.amount;
    let fee_bps = escrow.transfer_fee_bps as u128;
    let fee_amount = (amount as u128)
        .checked_mul(fee_bps)
        .and_then(|v| v.checked_div(10_000))
        .and_then(|v| u64::try_from(v).ok())
        .ok_or(EscrowError::FeeCalculationError)?;

    let net_amount = amount.checked_sub(fee_amount).ok_or(EscrowError::Overflow)?;

    require!(net_amount > 0, EscrowError::InvalidAmount);

    // TODO: Add Merkle allowlist/blacklist verification here later

    // === PDA signer seeds for CPI ===
    let seeds: &[&[u8]] = &[
        b"escrow",
        escrow.maker.as_ref(),
        escrow.taker.as_ref(),
        escrow.stablecoin_mint.as_ref(),
        &[escrow.bump],
    ];
    let signer_seeds = &[&seeds[..]];

    // === Transfer net amount to taker ===
    
    // ── 3. Transfer net amount to taker ────────────────────
    let transfer_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        TransferChecked {
            from: ctx.accounts.escrow_vault.to_account_info(),
            mint: ctx.accounts.stablecoin_mint.to_account_info(),
            to: ctx.accounts.taker_token_account.to_account_info(),
            authority: ctx.accounts.escrow_account.to_account_info(),
        },
        signer_seeds,
    );
    transfer_checked(transfer_ctx, net_amount, ctx.accounts.stablecoin_mint.decimals)?;

    // 🔄 Reload vault to get real on‑chain balance after the transfer
    ctx.accounts.escrow_vault.reload()?;
    let remaining = ctx.accounts.escrow_vault.amount;

    // ── 4. Sweep remaining tokens (dust/fee) to maker ─────
    if remaining > 0 {
        let sweep_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            TransferChecked {
                from: ctx.accounts.escrow_vault.to_account_info(),
                mint: ctx.accounts.stablecoin_mint.to_account_info(),
                to: ctx.accounts.maker_token_account.to_account_info(),
                authority: ctx.accounts.escrow_account.to_account_info(),
            },
            signer_seeds,
        );
        transfer_checked(sweep_ctx, remaining, ctx.accounts.stablecoin_mint.decimals)?;
    }

    // ── 5. Close the vault (balance now exactly 0) ─────────
    let close_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        CloseAccount {
            account: ctx.accounts.escrow_vault.to_account_info(),
            destination: ctx.accounts.maker_ref.to_account_info(),
            authority: ctx.accounts.escrow_account.to_account_info(),
        },
        signer_seeds,
    );
    close_account(close_ctx)?;

    // escrow_account automatically closed by `close = maker_ref`

    emit!(EscrowTransferred {
        maker: escrow.maker,
        taker: escrow.taker,
        net_amount,
        fee_amount,
    });

    Ok(())
}

#[event]
pub struct EscrowTransferred {
    pub maker: Pubkey,
    pub taker: Pubkey,
    pub net_amount: u64,
    pub fee_amount: u64,
}
