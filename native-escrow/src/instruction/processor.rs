// Step 1: Import AccountInfo struct representing account metadata and pointers to data.
use solana_program::account_info::AccountInfo;
// Step 2: Import ProgramResult representing the standard return type of Solana entry points.
use solana_program::entrypoint::ProgramResult;
// Step 3: Import the Pubkey type representing 32-byte public keys of accounts/programs.
use solana_program::pubkey::Pubkey;
// Step 4: Import the EscrowInstruction enum to map bytecode to instruction handlers.
use crate::state::EscrowInstruction;

// Step 5: Define the main process_instruction handler function which routes execution paths.
pub fn process_instruction(
    // Step 6: Accept the program ID under which this program is executing.
    program_id: &Pubkey,
    // Step 7: Accept the list of accounts referenced in this instruction.
    accounts: &[AccountInfo],
    // Step 8: Accept the instruction-specific data serialized as a byte slice.
    instruction_data: &[u8],
) -> ProgramResult {
    // Step 9: Validate that instruction data is not empty (needs at least a 1-byte discriminator).
    if instruction_data.is_empty() {
        // Step 10: Return an InvalidInstructionData error if the slice is empty.
        return Err(crate::errors::EscrowError::InvalidInstructionData.into());
    }

    // Step 11: Decode the instruction enum using the first byte of instruction data.
    let instruction = EscrowInstruction::from_u8(instruction_data)?;

    // Step 12: Match the parsed instruction to its specific handler module.
    match instruction {
        // Step 13: Handle EscrowInstruction::Initialize instruction.
        EscrowInstruction::Initialize => {
            // Step 14: Route to the initialize function, passing the remaining payload bytes.
            crate::instruction::initialize::initialize(
                program_id,
                accounts,
                &instruction_data[1..],
            )
        }
        // Step 15: Handle EscrowInstruction::ExecuteTransfer instruction.
        EscrowInstruction::ExecuteTransfer => {
            // Step 16: Route to the execute_transfer function, passing the remaining payload bytes.
            crate::instruction::execute_transfer::execute_transfer(
                program_id,
                accounts,
                &instruction_data[1..],
            )
        }
        // Step 17: Handle EscrowInstruction::CancelEscrow instruction.
        EscrowInstruction::CancelEscrow => {
            // Step 18: Route to the cancel_escrow function, passing the remaining payload bytes.
            crate::instruction::cancel_escrow::cancel_escrow(
                program_id,
                accounts,
                &instruction_data[1..],
            )
        }
    }
}