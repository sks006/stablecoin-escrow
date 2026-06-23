// O(1) zero-allocation evaluation engine placeholder
use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    pubkey::Pubkey,
    msg,
    program_error::ProgramError,
};
use crate::{
    state::{EscrowInstruction, NativeEscrowState},
    errors::EscrowError,
};

/// Main instruction dispatcher - zero-allocation router
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    msg!("NativeEscrow: Processing zero-copy instruction");

    // Early validation
    if instruction_data.is_empty() {
        return Err(crate::errors::EscrowError::InvalidInstructionData.into());
    }

    // Enterprise enum dispatch
    let instruction = EscrowInstruction::from_u8(instruction_data)?;

    // Route to typed handlers (remaining data passed for params)
    match instruction {
        EscrowInstruction::Initialize => {
            crate::instruction::initialize::initialize(
                program_id,
                accounts,
                &instruction_data[1..],
            )
        }
        EscrowInstruction::ExecuteTransfer => {
            crate::instruction::execute_transfer::execute_transfer(
                program_id,
                accounts,
                &instruction_data[1..],
            )
        }
        EscrowInstruction::CancelEscrow => {
            crate::instruction::cancel_escrow::cancel_escrow(
                program_id,
                accounts,
                &instruction_data[1..],
            )
        }
    }
}