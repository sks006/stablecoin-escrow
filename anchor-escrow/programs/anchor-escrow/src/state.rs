// Dynamic heap state layouts placeholder

use anchor_lang::prelude::*;


#[account]
#[derive(InitSpace)]
pub struct AnchorStablecoinEscrow {

    pub maker: Pubkey,
    pub taker: Pubkey,
    pub stablecoin_mint: Pubkey,
    pub amount: u64,
    pub transfer_fee_bps: u16,   // Applied on release to receiver
    pub allowlist_merkle_root: [u8; 32],
    pub blacklist_merkle_root: [u8; 32],
    pub bump: u8, // bump for PDA
    pub is_initialized: bool,
    pub cancel_authority: Pubkey,   // ← added
}

