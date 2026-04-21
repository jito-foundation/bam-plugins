use core::mem::size_of;

use pinocchio::{AccountView, Address, ProgramResult};
use solana_program_error::ProgramError;
use wincode::{Deserialize as _, SchemaRead};

use crate::{
    instructions::utils::verify_upgrade_authority_or_delegate_or_override_authority,
    state::{MemCmp, ProgramConfig},
};

const IXN_DATA_LEN: usize = size_of::<UpdateProgramMemcmpInstructionData>();

#[repr(C)]
#[derive(SchemaRead)]
struct UpdateProgramMemcmpInstructionData {
    offset: u16,
    length: u16,
}

fn checks(
    program_id: &Address,
    authority: &AccountView,
    program_config: &AccountView,
    config: &AccountView,
    program_account: &AccountView,
    executable_data: &AccountView,
    data: &[u8],
) -> ProgramResult {
    if !authority.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if !program_config.owned_by(program_id) || !config.owned_by(program_id) {
        return Err(ProgramError::InvalidAccountOwner);
    }

    verify_upgrade_authority_or_delegate_or_override_authority(
        authority,
        Some(config),
        program_config,
        program_account,
        executable_data,
    )?;

    if data.len() != IXN_DATA_LEN {
        return Err(ProgramError::InvalidInstructionData);
    }

    deserialize_instruction_data(data)?;

    Ok(())
}

/// Accounts:
///   0. `[signer]`   authority (must be the upgrade authority, delegate authority, or override authority)
///   1. `[writable]` program_config
///   2. `[]`         config
///   3. `[]`         program_account
///   4. `[]`         executable_data
///
/// Instruction data:
/// `[offset: u16, length: u16]`
pub fn process(program_id: &Address, accounts: &[AccountView], data: &[u8]) -> ProgramResult {
    let [authority, program_config, config, program_account, executable_data] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    checks(
        program_id,
        authority,
        program_config,
        config,
        program_account,
        executable_data,
        data,
    )?;

    let instruction_data = deserialize_instruction_data(data)?;
    let mut program_config_data = program_config.try_borrow_mut()?;
    let program_config_state = ProgramConfig::load_mut(&mut program_config_data)?;
    program_config_state.seqno_instruction_data_offset = MemCmp {
        offset: instruction_data.offset,
        length: instruction_data.length,
    };

    Ok(())
}

fn deserialize_instruction_data(
    data: &[u8],
) -> Result<UpdateProgramMemcmpInstructionData, ProgramError> {
    UpdateProgramMemcmpInstructionData::deserialize(data)
        .map_err(|_| ProgramError::InvalidInstructionData)
}
