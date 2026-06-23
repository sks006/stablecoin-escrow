// #[repr(C)] + Bytemuck unaligned layouts placeholder
use bytemuck::{Pod, Zeroable};
use solana_program::pubkey::Pubkey;

/// Fixed‑size, inline representation of a token account’s base fields.
/// Directly mappable via `bytemuck::from_bytes`.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct TokenAccountBase {
    pub mint: [u8; 32],
    pub owner: [u8; 32],
    pub amount: [u8; 8],
    pub delegate: [u8; 36],
    pub state: u8,
    pub is_native: [u8; 12],
    pub delegated_amount: [u8; 8],
    pub close_authority: [u8; 36],
}

/// Escrow state stored on‑chain. Mirrors `AnchorStablecoinEscrow`.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct NativeEscrowState {
    pub maker: Pubkey,                     // 32
    pub taker: Pubkey,                     // 32
    pub stablecoin_mint: Pubkey,           // 32
    pub amount: u64,                       // 8
    pub transfer_fee_bps: u16,             // 2
    pub allowlist_merkle_root: [u8; 32],   // 32
    pub blacklist_merkle_root: [u8; 32],   // 32
    pub bump: u8,                          // 1
    pub is_initialized: u8,                // 1 (0 = false, 1 = true)
    pub cancel_authority: Pubkey,          // 32
    _padding: [u8; 4],                        // 4 (to align to 8 bytes)
}
// total = 32*6 + 8 + 2 + 1 + 1 = 192 + 8 + 2 + 1 + 1 = 204 bytes this is the final struct

impl NativeEscrowState {
    pub const DISCRIMINATOR: [u8; 8] = *b"escrowv1"; // 8 bytes
    pub const LEN:usize =std::mem::size_of::<Self>(); // 204 bytes
    }


// Enterprise - grade Instruction Enum for the Escrow Program

#[repr(u8)]
#[derive(Debug,PartialEq)]
pub enum EscrowInstruction {
    Initialize,
    ExecuteTransfer,
    CancelEscrow,
}

impl EscrowInstruction {
    pub fn from_u8(input: &[u8]) ->Result<Self, crate::errors::EscrowError> {
        if input.is_empty() {
            return Err(crate::errors::EscrowError::InvalidInstructionData);
        }
        match input[0] {
            0 => Ok(EscrowInstruction::Initialize),
            1 => Ok(EscrowInstruction::ExecuteTransfer),
            2 => Ok(EscrowInstruction::CancelEscrow),
            _ => Err(crate::errors::EscrowError::InvalidInstructionData),
        }
    }
}
