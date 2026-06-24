// Step 1: Declare the entrypoint module to expose the Solana program entry point function.
pub mod entrypoint;
// Step 2: Declare the errors module to define custom program errors.
pub mod errors;
// Step 3: Declare the instruction module containing the instruction enum and instruction handlers.
pub mod instruction;
// Step 4: Declare the state module defining the layout of the escrow state account and helper structures.
pub mod state;

// Step 5: Re-export EscrowError at the crate root for easier importing across the program.
pub use errors::EscrowError;
// Step 6: Re-export NativeEscrowState at the crate root to make it accessible to other modules.
pub use state::NativeEscrowState;