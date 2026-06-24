// Step 1: Import the standard ProgramError type to define custom errors returned by the program.
use solana_program::program_error::ProgramError;

// Step 2: Implement the zero-allocation error enum, with representation as u32.
#[derive(Debug, Clone, Copy)]
#[repr(u32)]
pub enum EscrowError {
    // Step 3: Define the InvalidInstruction error for unsupported/malformed instruction discriminators.
    InvalidInstruction = 6000,
    // Step 4: Define the InvalidInstructionData error when instruction_data slice has incorrect bounds/types.
    InvalidInstructionData = 6001,
    // Step 5: Define the InvalidAmount error when a requested transfer amount is zero or invalid.
    InvalidAmount = 6002,
    // Step 6: Define the InvalidFeeBps error when the specified fee basis points exceed the max (10000).
    InvalidFeeBps = 6003,
    // Step 7: Define the InsufficientBalance error if the maker token account balance is too low.
    InsufficientBalance = 6004,
    // Step 8: Define the FeeCalculationError if multiplication/division overflows or fails mathematically.
    FeeCalculationError = 6005,
    // Step 9: Define the Overflow error when standard u64 mathematical operations exceed boundaries.
    Overflow = 6006,
    // Step 10: Define the UnauthorizedTaker error when the signer does not match the designated taker.
    UnauthorizedTaker = 6007,
    // Step 11: Define the NotInitialized error if trying to interact with an uninitialized escrow account.
    NotInitialized = 6008,
    // Step 12: Define the InvalidMerkleProof error if the Merkle proof does not resolve to the state root.
    InvalidMerkleProof = 6009,
    // Step 13: Define the UnauthorizedCanceller error if cancel_authority does not match the signer.
    UnauthorizedCanceller = 6010,
    // Step 14: Define the InvalidAmountAfterFee error if fee subtraction results in 0 net tokens.
    InvalidAmountAfterFee = 6011,
    // Step 15: Define the InvalidAuthority error if a required signer flag is false in the accounts.
    InvalidAuthority = 6012,
    // Step 16: Define the InvalidAccountOwner error if the account is not owned by the escrow program ID.
    InvalidAccountOwner = 6013,
    // Step 17: Define the InvalidTokenProgram error if the token program is not Token-2022.
    InvalidTokenProgram = 6014,
    // Step 18: Define the InvalidAccountData error if account length is smaller than the required layout.
    InvalidAccountData = 6015,
    // Step 19: Define the InvalidEscrowPDA error if the derived Escrow PDA key doesn't match the passed one.
    InvalidEscrowPDA = 6016,
    // Step 20: Define the InvalidVaultPDA error if the derived Vault PDA key doesn't match the passed one.
    InvalidVaultPDA = 6017,
    // Step 21: Define the InvalidTaker error if the taker account passed doesn't match the state's taker.
    InvalidTaker = 6018,
    // Step 22: Define the InvalidMint error if the stablecoin mint address doesn't match the state.
    InvalidMint = 6019,
    // Step 23: Define the AlreadyInitialized error if initializing an escrow that is already initialized.
    AlreadyInitialized = 6020,
    // Step 24: Define the UninitializedAccount error if a token account is not initialized.
    UninitializedAccount = 6021,
}

// Step 25: Implement From trait to allow easy conversion from EscrowError to ProgramError via the ? operator.
impl From<EscrowError> for ProgramError {
    // Step 26: Define the conversion function.
    fn from(e: EscrowError) -> Self {
        // Step 27: Cast the EscrowError enum to its u32 integer representation and wrap in Custom.
        ProgramError::Custom(e as u32)
    }
}