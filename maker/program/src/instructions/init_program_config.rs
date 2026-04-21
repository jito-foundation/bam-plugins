use pinocchio::{
    cpi::{Seed, Signer},
    AccountView, Address, ProgramResult,
};
use pinocchio_system::create_account_with_minimum_balance_signed;
use solana_program_error::ProgramError;
use wincode::Deserialize as _;

use crate::{
    instructions::utils::verify_upgrade_authority_or_override_authority,
    state::{MarketUpdateMode, ProgramConfig, ProgramStatus, PDA_SEED_COUNT, PROGRAM_CONFIG_SIZE},
};

wincode::pod_wrapper! {
    unsafe struct InstructionPodAddress(Address);
}

#[repr(C)]
#[derive(wincode::SchemaRead)]
struct InitProgramConfigData {
    #[wincode(with = "InstructionPodAddress")]
    target_program_id: Address,
    bump: u8,
    market_update_mode: u8,
}

const IXN_DATA_LEN: usize = core::mem::size_of::<InitProgramConfigData>();

// Checks:
//   - payer is a signer
//   - instruction data is exactly IXN_DATA_LEN bytes and deserializes successfully
//   - program_config address matches the PDA derived from seeds [target_program_id, bump]
//   - program_account address matches the target_program_id from instruction data
//   - config is owned by this program
//   - payer is the upgrade authority or config.override_authority (via util)
//   returns the (target_program_id, bump, market_update_mode) from instruction data
fn checks(
    program_id: &Address,
    payer: &AccountView,
    program_config: &AccountView,
    config: &AccountView,
    program_account: &AccountView,
    executable_data: &AccountView,
    data: &[u8],
) -> Result<(Address, u8, MarketUpdateMode), ProgramError> {
    if !payer.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if data.len() != IXN_DATA_LEN {
        return Err(ProgramError::InvalidInstructionData);
    }

    let instruction_data = InitProgramConfigData::deserialize(data)
        .map_err(|_| ProgramError::InvalidInstructionData)?;
    let target_program_id = instruction_data.target_program_id;
    let bump = instruction_data.bump;
    let market_update_mode = MarketUpdateMode::from_u8(instruction_data.market_update_mode)?;

    let expected = pinocchio_pubkey::derive_address(
        &[target_program_id.as_ref()],
        Some(bump),
        program_id.as_array(),
    );
    if program_config.address() != &Address::from(expected) {
        return Err(ProgramError::InvalidSeeds);
    }

    if program_account.address() != &target_program_id {
        return Err(ProgramError::InvalidArgument);
    }

    if !config.owned_by(program_id) {
        return Err(ProgramError::InvalidAccountOwner);
    }

    verify_upgrade_authority_or_override_authority(
        payer,
        config,
        program_account,
        executable_data,
    )?;

    Ok((target_program_id, bump, market_update_mode))
}

/// Accounts:
///   0. `[signer, writable]` payer (must be the upgrade authority or override authority)
///   1. `[writable]`         program_config (PDA: seeds = [target_program_id, bump])
///   2. `[]`                 config
///   3. `[]`                 program_account (address must equal target_program_id)
///   4. `[]`                 executable_data
///   5. `[]`                 system_program
///   6. `[]`                 rent_sysvar
///
/// Instruction data: `[target_program_id: Address, bump: u8, market_update_mode: u8]` (34 bytes)
pub fn process(program_id: &Address, accounts: &[AccountView], data: &[u8]) -> ProgramResult {
    let [payer, program_config, config, program_account, executable_data, _system_program, rent_sysvar] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    let (target_program_id, bump, market_update_mode) = checks(
        program_id,
        payer,
        program_config,
        config,
        program_account,
        executable_data,
        data,
    )?;

    let bump_seed = [bump];
    let seeds: [Seed; PDA_SEED_COUNT] = [
        Seed::from(target_program_id.as_ref()),
        Seed::from(&bump_seed),
    ];
    let signer = Signer::from(&seeds);

    create_account_with_minimum_balance_signed(
        program_config,
        PROGRAM_CONFIG_SIZE,
        program_id,
        payer,
        Some(rent_sysvar),
        &[signer],
    )?;

    {
        let mut program_config_data = program_config.try_borrow_mut()?;
        let program_config_state = ProgramConfig::load_mut(&mut program_config_data)?;
        program_config_state.program_id = target_program_id;
        program_config_state.status = ProgramStatus::Enrolled;
        program_config_state.market_update_mode = market_update_mode;
    }

    Ok(())
}
