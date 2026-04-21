use pinocchio::{AccountView, Address, ProgramResult};
use solana_program_error::ProgramError;

use crate::{
    instructions::utils::verify_upgrade_authority_or_delegate_or_override_authority,
    state::{ProgramConfig, ProgramStatus},
};

// Checks:
//   - authority is a signer
//   - program_config and config are owned by this program
//   - authority is the upgrade authority, delegate authority, or override authority (via util)
fn checks(
    program_id: &Address,
    authority: &AccountView,
    config: &AccountView,
    program_account: &AccountView,
    executable_data: &AccountView,
    program_config: &AccountView,
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
    )
}

/// Accounts:
///   0. `[signer]`   authority (must match upgrade authority, delegate authority, or override authority)
///   1. `[]`         config
///   2. `[]`         program_account
///   3. `[]`         executable_data
///   4. `[writable]` program_config
///
/// Instruction data: none
pub fn process(program_id: &Address, accounts: &[AccountView], _data: &[u8]) -> ProgramResult {
    let [authority, config, program_account, executable_data, program_config] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    checks(
        program_id,
        authority,
        config,
        program_account,
        executable_data,
        program_config,
    )?;

    {
        let mut program_config_data = program_config.try_borrow_mut()?;
        let program_config_state = ProgramConfig::load_mut(&mut program_config_data)?;
        program_config_state.status = ProgramStatus::Unenroll;
    }

    Ok(())
}
