use anchor_lang::prelude::*;

#[error_code]
pub enum EscrowError {
    #[msg("Address not in allowlist")]
    NotAllowed,
    #[msg("Address is blacklisted")]
    Blacklisted,
    #[msg("Merkle proof verification failed")]
    InvalidMerkleProof,
    #[msg("Arithmetic overflow")]
    Overflow,
    #[msg("Only the designated taker can execute this escrow")]
    UnauthorizedTaker,
    #[msg("Invalid token program")]
    InvalidTokenProgram,
    #[msg("Amount must be greater than zero")]
    InvalidAmount,
    #[msg("Transfer fee basis points must be <= 10,000")]
    InvalidFeeBps,
    #[msg("Maker has insufficient token balance")]
    InsufficientBalance,
    #[msg("Escrow account is not initialized")]
    NotInitialized,
    #[msg("Only the designated cancel authority can cancel this escrow")]
    UnauthorizedCanceller,
    #[msg("Fee calculation error")]
    FeeCalculationError,
}