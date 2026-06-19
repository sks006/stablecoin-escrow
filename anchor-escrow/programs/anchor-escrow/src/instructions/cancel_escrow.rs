use anchor_lang::prelude::*;
use anchor_spl::token_interface::{
    transfer_checked, close_account, Mint, TokenAccount, TokenInterface,
    TransferChecked, CloseAccount,
};
use crate::errors::EscrowError;
use crate::state::AnchorStablecoinEscrow;

#[derive(Accounts)]
#[instruction(taker: Pubkey)] // taker is needed for PDA seeds, but not a signer here
pub struct CancelEscrow<'info> {
    /// Only the stored cancel_authority can cancel

    pub cancel_authority: Signer<'info>,

    #[account(
        mut,
        token::mint = stablecoin_mint,
        token::authority = maker_ref,
    )]
    pub maker_token_account: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        constraint = escrow_account.cancel_authority == *cancel_authority.key @ EscrowError::UnauthorizedCanceller,
        constraint = escrow_account.is_initialized @ EscrowError::NotInitialized,
        seeds = [
            b"escrow",
            maker_ref.key().as_ref(),
            taker.as_ref(),
            stablecoin_mint.key().as_ref(),
        ],
        bump = escrow_account.bump,
        close = maker_ref,
    )]
    pub escrow_account: Account<'info, AnchorStablecoinEscrow>,

    #[account(
        mut,
        token::mint = stablecoin_mint,
        token::authority = escrow_account,
    )]
    pub escrow_vault: InterfaceAccount<'info, TokenAccount>,

    /// CHECK: rent refund destination
    #[account(mut)]
    pub maker_ref: UncheckedAccount<'info>,

    pub stablecoin_mint: InterfaceAccount<'info, Mint>,
    pub token_program: Interface<'info, TokenInterface>,
}

pub fn handler(ctx: Context<CancelEscrow>, taker: Pubkey) -> Result<()> {
    let escrow = &ctx.accounts.escrow_account;

    // PDA signer seeds (must include bump)
 let maker_ref_key = ctx.accounts.maker_ref.key();
let mint_key = ctx.accounts.stablecoin_mint.key();

let seeds: &[&[u8]] = &[
    b"escrow",
    maker_ref_key.as_ref(),
    taker.as_ref(),
    mint_key.as_ref(),
    &[escrow.bump],
];
let signer_seeds = &[&seeds[..]];

    // Sweep entire vault balance back to maker (no fee)
    let vault_balance = ctx.accounts.escrow_vault.amount;
    if vault_balance > 0 {
        let transfer_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            TransferChecked {
                from: ctx.accounts.escrow_vault.to_account_info(),
                mint: ctx.accounts.stablecoin_mint.to_account_info(),
                to: ctx.accounts.maker_token_account.to_account_info(),
                authority: ctx.accounts.escrow_account.to_account_info(),
            },
            signer_seeds,
        );
        transfer_checked(transfer_ctx, vault_balance, ctx.accounts.stablecoin_mint.decimals)?;
    }

    // Close vault (now zero balance)
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

    // escrow_account is auto-closed by `close = maker_ref`
    Ok(())
}