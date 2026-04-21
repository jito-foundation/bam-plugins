use pinocchio::{AccountView, Address, ProgramResult};
use solana_program_error::ProgramError;
use wincode::Deserialize as _;

use crate::{
    instructions::utils::verify_upgrade_authority_or_override_authority, state::ProgramConfig,
};

wincode::pod_wrapper! {
    unsafe struct InstructionPodAddress(Address);
}

#[repr(C)]
#[derive(wincode::SchemaRead)]
struct AssignDelegateAuthorityInstructionData {
    #[wincode(with = "InstructionPodAddress")]
    delegate_authority: Address,
}

const IXN_DATA_LEN: usize = core::mem::size_of::<AssignDelegateAuthorityInstructionData>();

// Checks:
//   - authority is a signer
//   - config and program_config are owned by this program
//   - instruction data is exactly IXN_DATA_LEN bytes and deserializes successfully
//   - program_account address matches program_config.program_id
//   - authority is the upgrade authority or config.override_authority (via util)
//   returns the deserialized delegate_authority address
fn checks(
    program_id: &Address,
    authority: &AccountView,
    config: &AccountView,
    program_config: &AccountView,
    program_account: &AccountView,
    executable_data: &AccountView,
    data: &[u8],
) -> Result<Address, ProgramError> {
    if !authority.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if !config.owned_by(program_id) || !program_config.owned_by(program_id) {
        return Err(ProgramError::InvalidAccountOwner);
    }

    if data.len() != IXN_DATA_LEN {
        return Err(ProgramError::InvalidInstructionData);
    }

    let instruction_data = AssignDelegateAuthorityInstructionData::deserialize(data)
        .map_err(|_| ProgramError::InvalidInstructionData)?;
    let delegate_authority = instruction_data.delegate_authority;

    let program_config_data = program_config.try_borrow()?;
    let program_config_state = ProgramConfig::load(&program_config_data)?;
    if program_account.address() != &program_config_state.program_id {
        return Err(ProgramError::InvalidArgument);
    }

    verify_upgrade_authority_or_override_authority(
        authority,
        config,
        program_account,
        executable_data,
    )?;

    Ok(delegate_authority)
}

/// Accounts:
///   0. `[signer]`   authority (must be the upgrade authority or config.override_authority)
///   1. `[]`         config
///   2. `[writable]` program_config
///   3. `[]`         program_account
///   4. `[]`         executable_data
///
/// Instruction data: `[delegate_authority: Address]`
pub fn process(program_id: &Address, accounts: &[AccountView], data: &[u8]) -> ProgramResult {
    let [authority, config, program_config, program_account, executable_data] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    let delegate_authority = checks(
        program_id,
        authority,
        config,
        program_config,
        program_account,
        executable_data,
        data,
    )?;

    let mut program_config_data = program_config.try_borrow_mut()?;
    let program_config_state = ProgramConfig::load_mut(&mut program_config_data)?;
    program_config_state.delegate_authority = delegate_authority;

    Ok(())
}
