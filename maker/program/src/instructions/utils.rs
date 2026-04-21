use bincode::deserialize;
use pinocchio::{AccountView, Address, ProgramResult};
use solana_loader_v3_interface::state::UpgradeableLoaderState;
use solana_program_error::ProgramError;

use crate::{
    state,
    state::{is_default_address, Config, ProgramConfig},
};

/// Verifies that `authority` is the upgrade authority of `program_account` by
/// deserializing the BPF Upgradeable Loader account chain.
pub fn verify_upgrade_authority(
    authority: &AccountView,
    program_account: &AccountView,
    executable_data: &AccountView,
) -> ProgramResult {
    let loader_id = Address::from(solana_sdk_ids::bpf_loader_upgradeable::id().to_bytes());
    if !program_account.owned_by(&loader_id) || !executable_data.owned_by(&loader_id) {
        return Err(ProgramError::InvalidAccountOwner);
    }

    let prog_data = program_account.try_borrow()?;
    let program_state: UpgradeableLoaderState =
        deserialize(&prog_data).map_err(|_| ProgramError::InvalidAccountData)?;
    let expected_executable_data = match program_state {
        UpgradeableLoaderState::Program {
            programdata_address,
        } => Address::from(programdata_address.to_bytes()),
        _ => return Err(ProgramError::InvalidAccountData),
    };
    if executable_data.address() != &expected_executable_data {
        return Err(ProgramError::InvalidAccountData);
    }
    drop(prog_data);

    let exec_data = executable_data.try_borrow()?;
    let executable_data_state: UpgradeableLoaderState =
        deserialize(&exec_data).map_err(|_| ProgramError::InvalidAccountData)?;
    let upgrade_authority = match executable_data_state {
        UpgradeableLoaderState::ProgramData {
            upgrade_authority_address: Some(upgrade_authority_address),
            ..
        } => Address::from(upgrade_authority_address.to_bytes()),
        _ => return Err(ProgramError::InvalidAccountData),
    };
    if authority.address() != &upgrade_authority {
        return Err(ProgramError::MissingRequiredSignature);
    }

    Ok(())
}

pub fn verify_upgrade_authority_or_override_authority(
    authority: &AccountView,
    config: &AccountView,
    program_account: &AccountView,
    executable_data: &AccountView,
) -> ProgramResult {
    let config_data = config.try_borrow()?;
    if config_data.len() < state::CONFIG_SIZE {
        return Err(ProgramError::InvalidAccountData);
    }

    let config_state = Config::load(&config_data)?;
    let override_authority = config_state.override_authority;
    drop(config_data);

    if authority.address() == &override_authority {
        return Ok(());
    }

    verify_upgrade_authority(authority, program_account, executable_data)
}

pub fn verify_upgrade_authority_or_delegate_or_override_authority(
    authority: &AccountView,
    config: Option<&AccountView>,
    program_config: &AccountView,
    program_account: &AccountView,
    executable_data: &AccountView,
) -> ProgramResult {
    let program_config_data = program_config.try_borrow()?;
    if program_config_data.len() < state::PROGRAM_CONFIG_SIZE {
        return Err(ProgramError::InvalidAccountData);
    }

    let program_config_state = ProgramConfig::load(&program_config_data)?;
    let program_id = program_config_state.program_id;
    let delegate_authority = program_config_state.delegate_authority;
    drop(program_config_data);

    if program_account.address() != &program_id {
        return Err(ProgramError::InvalidArgument);
    }

    if let Some(config) = config {
        let config_data = config.try_borrow()?;
        if config_data.len() < state::CONFIG_SIZE {
            return Err(ProgramError::InvalidAccountData);
        }

        let config_state = Config::load(&config_data)?;
        if authority.address() == &config_state.override_authority {
            return Ok(());
        }
    }

    if authority.address() == &delegate_authority && !is_default_address(&delegate_authority) {
        return Ok(());
    }

    verify_upgrade_authority(authority, program_account, executable_data)
}
