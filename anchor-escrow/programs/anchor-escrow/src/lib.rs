use anchor_lang::prelude::*;

// Bring your modules into scope
pub mod state;
pub mod errors;
pub mod instructions;

use instructions::*;

// 🛑 Hardware Addressing: This ID must exactly match your Anchor.toml and your local keypair.
// Run `anchor keys sync` after your first build to update this automatically.
declare_id!("B5KMPtmn3yB2P7sqmDjQXvrFYcDfbi7U5L6DsCKgqF6u"); 

#[program]
pub mod anchor_escrow {
    use super::*;

    pub fn initialize_escrow(
        ctx: Context<InitializeEscrow>,
        amount: u64,
        transfer_fee_bps: u16,
        allowlist_merkle_root: [u8; 32],
        blacklist_merkle_root: [u8; 32],
        taker: Pubkey,
        cancel_authority: Pubkey,
    ) -> Result<()> {
        instructions::initialize::handler(
            ctx, 
            amount, 
            transfer_fee_bps, 
            allowlist_merkle_root, 
            blacklist_merkle_root, 
            taker, 
            cancel_authority
        )
    }

    pub fn execute_transfer(ctx: Context<ExecuteTransfer>) -> Result<()> {
        instructions::execute_transfer::handler(ctx)
    }

    pub fn cancel_escrow(ctx: Context<CancelEscrow>, taker: Pubkey) -> Result<()> {
        instructions::cancel_escrow::handler(ctx, taker)
    }
}