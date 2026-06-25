// Step 1: Import bytemuck's Pod and Zeroable traits for safe zero-copy casting of raw bytes to/from structs.
use bytemuck::{Pod, Zeroable};

// Step 2: Define TokenAccountBase to represent the memory layout of an SPL Token account.
// Step 3: Enforce C-compatible structure layout in memory.
#[repr(C)]
// Step 4: Derive Clone, Copy, and bytemuck traits (Pod and Zeroable) to enable casting from raw account data.
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct TokenAccountBase {
    // Step 5: Byte array representing the 32-byte Pubkey of the token mint.
    pub mint: [u8; 32],                  // 32 bytes
    // Step 6: Byte array representing the 32-byte Pubkey of the token account owner.
    pub owner: [u8; 32],                 // 32 bytes
    // Step 7: Byte array representing the 8-byte (u64) amount of tokens in the account.
    pub amount: [u8; 8],                 // 8 bytes
    // Step 8: COption equivalent: 4 bytes representing if there is a delegate authority (COption: 0 = None, 1 = Some).
    pub delegate_option: [u8; 4],        // 4 bytes
    // Step 9: Byte array representing the 32-byte Pubkey of the delegate authority.
    pub delegate_key: [u8; 32],          // 32 bytes
    // Step 10: Byte representing token account state (0 = Uninitialized, 1 = Initialized, 2 = Frozen).
    pub state: u8,                       // 1 byte
    // Step 11: COption equivalent: 4 bytes representing if is_native is set.
    pub is_native_option: [u8; 4],       // 4 bytes
    // Step 12: Byte array representing the 8-byte (u64) native amount (if applicable).
    pub is_native: [u8; 8],              // 8 bytes
    // Step 13: Byte array representing the 8-byte (u64) delegated amount.
    pub delegated_amount: [u8; 8],       // 8 bytes
    // Step 14: COption equivalent: 4 bytes representing if there is a close authority.
    pub close_authority_option: [u8; 4], // 4 bytes
    // Step 15: Byte array representing the 32-byte Pubkey of the close authority.
    pub close_authority_key: [u8; 32],   // 32 bytes
}

// Step 16: Define NativeEscrowState to store the escrow program configuration and state.
// Step 17: Enforce C-compatible structure layout to ensure fields have predictable, consistent offsets.
#[repr(C)]
// Step 18: Derive Clone, Copy, and bytemuck traits (Pod and Zeroable) to enable casting from raw account data.
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct NativeEscrowState {
    // Step 19: 32-byte array representing the Pubkey of the escrow maker (creator).
    pub maker: [u8; 32],                 // 32 bytes
    // Step 20: 32-byte array representing the Pubkey of the escrow taker (allowed buyer).
    pub taker: [u8; 32],                 // 32 bytes
    // Step 21: 32-byte array representing the Pubkey of the stablecoin Mint.
    pub stablecoin_mint: [u8; 32],       // 32 bytes
    // Step 22: 32-byte array representing the Merkle Root of whitelisted addresses.
    pub allowlist_merkle_root: [u8; 32], // 32 bytes
    // Step 23: 32-byte array representing the Merkle Root of blacklisted addresses.
    pub blacklist_merkle_root: [u8; 32], // 32 bytes
    // Step 24: 32-byte array representing the Pubkey of the authority that can cancel this escrow.
    pub cancel_authority: [u8; 32],      // 32 bytes
    
    // Step 25: 8-byte (u64) field storing the amount of stablecoin tokens deposited in the escrow.
    pub amount: u64,                     // 8 bytes (offset 192, perfectly aligned)
    
    // Step 26: 2-byte (u16) field storing the transfer fee in basis points (1 = 0.01%).
    pub transfer_fee_bps: u16,           // 2 bytes
    // Step 27: 1-byte field storing the PDA bump seed for verifying the Escrow PDA.
    pub bump: u8,                        // 1 byte
    // Step 28: 1-byte flag storing if the state has been initialized (1 = yes, 0 = no).
    pub is_initialized: u8,              // 1 byte
    // Step 29: 4-byte padding array to round the struct size to the next multiple of 8 (208 bytes total).
    pub _padding: [u8; 4],               // 4 bytes
}

// Step 30: Implement associated constants on the NativeEscrowState structure.
impl NativeEscrowState {
    // Step 31: Define the unique 8-byte state discriminator used to identify native escrow accounts.
    pub const DISCRIMINATOR: [u8; 8] = *b"escrowv1"; 
    // Step 32: Define the total size of the NativeEscrowState struct in bytes.
    pub const LEN: usize = std::mem::size_of::<Self>(); 
}

// Step 33: Define the EscrowInstruction enum to list all supported program instructions.
#[repr(u8)]
pub enum EscrowInstruction {
    // Step 34: Initialize instruction (variant 0) to set up the escrow state.
    Initialize,
    // Step 35: ExecuteTransfer instruction (variant 1) to settle the escrow and pay the taker.
    ExecuteTransfer,
    // Step 36: CancelEscrow instruction (variant 2) to cancel the escrow and retrieve deposited tokens.
    CancelEscrow,
}

// Step 37: Implement parser methods for the EscrowInstruction enum.
impl EscrowInstruction {
    // Step 38: Define a function to parse a u8 discriminator from incoming instruction data.
    pub fn from_u8(input: &[u8]) -> Result<Self, crate::errors::EscrowError> {
        // Step 39: If instruction data is empty, return an InvalidInstructionData error.
        if input.is_empty() {
            return Err(crate::errors::EscrowError::InvalidInstructionData);
        }
        // Step 40: Match the first byte as the instruction discriminator.
        match input[0] {
            // Step 41: Discriminator 0 maps to the Initialize instruction.
            0 => Ok(EscrowInstruction::Initialize),
            // Step 42: Discriminator 1 maps to the ExecuteTransfer instruction.
            1 => Ok(EscrowInstruction::ExecuteTransfer),
            // Step 43: Discriminator 2 maps to the CancelEscrow instruction.
            2 => Ok(EscrowInstruction::CancelEscrow),
            // Step 44: Any other discriminator value is invalid, returning an error.
            _ => Err(crate::errors::EscrowError::InvalidInstructionData),
        }
    }
}