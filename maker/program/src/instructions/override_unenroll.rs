use pinocchio::{AccountView, Address, ProgramResult};
use solana_program_error::ProgramError;

use crate::state::{Config, ProgramConfig, ProgramStatus};

// Checks:
//   - authority is a signer
//   - config and program_config are owned by this program
//   - authority address matches config.override_authority
fn checks(
    program_id: &Address,
    authority: &AccountView,
    config: &AccountView,
    program_config: &AccountView,
) -> ProgramResult {
    if !authority.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if !config.owned_by(program_id) || !program_config.owned_by(program_id) {
        return Err(ProgramError::InvalidAccountOwner);
    }

    let config_data = config.try_borrow()?;
    let config_state = Config::load(&config_data)?;

    if authority.address() != &config_state.override_authority {
        return Err(ProgramError::MissingRequiredSignature);
    }

    Ok(())
}

/// Accounts:
///   0. `[signer]`   authority (must match config.override_authority)
///   1. `[]`         config
///   2. `[writable]` program_config
///
/// Instruction data: none
pub fn process(program_id: &Address, accounts: &[AccountView], _data: &[u8]) -> ProgramResult {
    let [authority, config, program_config] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    checks(program_id, authority, config, program_config)?;

    {
        let mut program_config_data = program_config.try_borrow_mut()?;
        let program_config_state = ProgramConfig::load_mut(&mut program_config_data)?;
        program_config_state.status = ProgramStatus::OverrideUnenroll;
    }
    Ok(())
}
