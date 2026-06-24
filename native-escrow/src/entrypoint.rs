// Step 1: Import AccountInfo struct representing account metadata and pointers to data.
use solana_program::account_info::AccountInfo;
// Step 2: Import the entrypoint macro used to declare the program's main entry function.
use solana_program::entrypoint;
// Step 3: Import ProgramResult representing the standard return type of Solana entry points.
use solana_program::entrypoint::ProgramResult;
// Step 4: Import the Pubkey type representing 32-byte public keys of accounts/programs.
use solana_program::pubkey::Pubkey;

// Step 5: Declare the entry point of the Solana program using the entrypoint! macro.
entrypoint!(process_instruction);

// Step 6: Define the process_instruction function which handles incoming transaction instructions.
pub fn process_instruction(
    // Step 7: Receive the program ID under which this program is executing.
    program_id: &Pubkey,
    // Step 8: Receive the list of accounts referenced in the instruction.
    accounts: &[AccountInfo],
    // Step 9: Receive the instruction-specific data serialized as a byte slice.
    instruction_data: &[u8],
) -> ProgramResult {
    // Step 10: Dispatch the call directly to the processor module's router.
    crate::instruction::processor::process_instruction(program_id, accounts, instruction_data)
}