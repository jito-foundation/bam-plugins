use pinocchio::{
    cpi::{Seed, Signer},
    AccountView, Address, ProgramResult,
};
use pinocchio_system::create_account_with_minimum_balance_signed;
use solana_program_error::ProgramError;
use wincode::Deserialize as _;

use crate::state::{CONFIG_SIZE, IXN_DISCRIMINATOR_LEN, PDA_SEED_COUNT};

#[repr(C)]
#[derive(wincode::SchemaRead)]
struct InitConfigInstructionData {
    bump: u8,
}

const IXN_DATA_LEN: usize = IXN_DISCRIMINATOR_LEN;

// Checks:
//   - payer is a signer
//   - instruction data is exactly IXN_DATA_LEN bytes and deserializes successfully
//   - config address matches the PDA derived from seeds [b"config", bump]
//   returns the bump byte
fn checks(
    program_id: &Address,
    payer: &AccountView,
    config: &AccountView,
    data: &[u8],
) -> Result<u8, ProgramError> {
    if !payer.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if data.len() != IXN_DATA_LEN {
        return Err(ProgramError::InvalidInstructionData);
    }

    let instruction_data = InitConfigInstructionData::deserialize(data)
        .map_err(|_| ProgramError::InvalidInstructionData)?;
    let bump = instruction_data.bump;
    let expected =
        pinocchio_pubkey::derive_address(&[b"config".as_ref()], Some(bump), program_id.as_array());

    if config.address() != &Address::from(expected) {
        return Err(ProgramError::InvalidSeeds);
    }

    Ok(bump)
}

/// Accounts:
///   0. `[signer, writable]` payer
///   1. `[writable]`         config (PDA: seeds = [b"config", bump])
///   2. `[]`                 system_program
///   3. `[]`                 rent_sysvar
///
/// Instruction data: `[bump: u8]` (1 byte)
pub fn process(program_id: &Address, accounts: &[AccountView], data: &[u8]) -> ProgramResult {
    let [payer, config, _system_program, rent_sysvar] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    let bump = checks(program_id, payer, config, data)?;

    let bump_seed = [bump];
    let seeds: [Seed; PDA_SEED_COUNT] = [Seed::from(b"config"), Seed::from(&bump_seed)];
    let signer = Signer::from(&seeds);

    create_account_with_minimum_balance_signed(
        config,
        CONFIG_SIZE,
        program_id,
        payer,
        Some(rent_sysvar),
        &[signer],
    )?;

    Ok(())
}
