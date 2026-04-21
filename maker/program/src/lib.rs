extern crate alloc;

use pinocchio::{entrypoint, AccountView, Address, ProgramResult};
use solana_program_error::ProgramError;

mod instructions;
pub mod state;

entrypoint!(process_instruction);

#[repr(u8)]
enum InstructionDiscriminant {
    InitConfig = 0,
    InitProgramConfig = 1,
    UpdateMarketConfig = 2,
    UpgradeAuthorityUnenroll = 3,
    Activate = 4,
    OverrideUnenroll = 5,
    AdminChangeAuthority = 6,
    AssignDelegateAuthority = 7,
    UpdateProgramSigner = 8,
    UpdateProgramMemcmp = 9,
}

impl InstructionDiscriminant {
    fn from_u8(value: u8) -> Result<Self, ProgramError> {
        match value {
            0 => Ok(Self::InitConfig),
            1 => Ok(Self::InitProgramConfig),
            2 => Ok(Self::UpdateMarketConfig),
            3 => Ok(Self::UpgradeAuthorityUnenroll),
            4 => Ok(Self::Activate),
            5 => Ok(Self::OverrideUnenroll),
            6 => Ok(Self::AdminChangeAuthority),
            7 => Ok(Self::AssignDelegateAuthority),
            8 => Ok(Self::UpdateProgramSigner),
            9 => Ok(Self::UpdateProgramMemcmp),
            _ => Err(ProgramError::InvalidInstructionData),
        }
    }
}

/// Instruction discriminants (first byte of instruction data).
/// Remaining bytes are instruction-specific arguments.
pub fn process_instruction(
    program_id: &Address,
    accounts: &[AccountView],
    instruction_data: &[u8],
) -> ProgramResult {
    let (discriminant, data) = instruction_data
        .split_first()
        .ok_or(ProgramError::InvalidInstructionData)?;

    match InstructionDiscriminant::from_u8(*discriminant)? {
        InstructionDiscriminant::InitConfig => {
            instructions::init_config::process(program_id, accounts, data)
        }
        InstructionDiscriminant::InitProgramConfig => {
            instructions::init_program_config::process(program_id, accounts, data)
        }
        InstructionDiscriminant::UpdateMarketConfig => {
            instructions::update_market_config::process(program_id, accounts, data)
        }
        InstructionDiscriminant::UpgradeAuthorityUnenroll => {
            instructions::upgrade_authority_unenroll::process(program_id, accounts, data)
        }
        InstructionDiscriminant::Activate => {
            instructions::activate::process(program_id, accounts, data)
        }
        InstructionDiscriminant::OverrideUnenroll => {
            instructions::override_unenroll::process(program_id, accounts, data)
        }
        InstructionDiscriminant::AdminChangeAuthority => {
            instructions::admin_change_authority::process(program_id, accounts, data)
        }
        InstructionDiscriminant::AssignDelegateAuthority => {
            instructions::assign_delegate_authority::process(program_id, accounts, data)
        }
        InstructionDiscriminant::UpdateProgramSigner => {
            instructions::update_program_signer::process(program_id, accounts, data)
        }
        InstructionDiscriminant::UpdateProgramMemcmp => {
            instructions::update_program_memcmp::process(program_id, accounts, data)
        }
    }
}

#[cfg(test)]
#[path = "../tests/unit/mod.rs"]
mod tests;
