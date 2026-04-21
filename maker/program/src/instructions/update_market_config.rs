use core::mem::size_of;

use pinocchio::{AccountView, Address, ProgramResult};
use solana_program_error::ProgramError;
use wincode::{Deserialize as _, SchemaRead};

use crate::{
    instructions::utils::verify_upgrade_authority_or_delegate_or_override_authority,
    state::{
        is_default_address, ProgramConfig, MARKET_CONFIG_COUNT, MARKET_ID_INDEX_LEN,
        MARKET_ID_PROGRAM_PREFIX_LEN,
    },
};

wincode::pod_wrapper! {
    unsafe struct InstructionPodAddress(Address);
}

const IXN_DATA_LEN: usize = size_of::<UpdateMarketConfigInstructionData>();

#[repr(C)]
#[derive(SchemaRead)]
struct UpdateMarketConfigInstructionData {
    index: u8,
    #[wincode(with = "InstructionPodAddress")]
    writable_account: Address,
}

// Checks:
//   - authority is a signer
//   - program_config and config are owned by this program
//   - authority is the upgrade authority, delegate authority, or override authority (via util)
//   - instruction data is exactly IXN_DATA_LEN bytes and deserializes successfully
//   - index is less than MARKET_CONFIG_COUNT
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

    let instruction_data = deserialize_instruction_data(data)?;
    if instruction_data.index as usize >= MARKET_CONFIG_COUNT {
        return Err(ProgramError::InvalidInstructionData);
    }

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
/// `[index: u8, writable_account: Address]`
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
    let market_config = &mut program_config_state.market_configs[instruction_data.index as usize];
    market_config.writable_account = instruction_data.writable_account;
    market_config.market_id = if is_default_address(&instruction_data.writable_account) {
        Address::default()
    } else {
        derive_market_id(program_account.address(), instruction_data.index)
    };

    Ok(())
}

fn deserialize_instruction_data(
    data: &[u8],
) -> Result<UpdateMarketConfigInstructionData, ProgramError> {
    UpdateMarketConfigInstructionData::deserialize(data)
        .map_err(|_| ProgramError::InvalidInstructionData)
}

// In derive_market_id we construct a unique market ID per writable account
// by combining the first MARKET_ID_PROGRAM_PREFIX_LEN (31) bytes of the program ID with the index.
// This gives us 248 bits of collision resistance from the program ID while supporting a cap of
// 255 unique market IDs per program.
fn derive_market_id(program_id: &Address, index: u8) -> Address {
    let mut market_id = [0u8; 32];
    let program_bytes = program_id.to_bytes();
    market_id[..MARKET_ID_PROGRAM_PREFIX_LEN]
        .copy_from_slice(&program_bytes[..MARKET_ID_PROGRAM_PREFIX_LEN]);
    market_id[MARKET_ID_PROGRAM_PREFIX_LEN..]
        .copy_from_slice(&(index as u16).to_be_bytes()[..MARKET_ID_INDEX_LEN]);
    market_id.into()
}
