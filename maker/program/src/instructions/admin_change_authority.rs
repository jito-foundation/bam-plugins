use pinocchio::{AccountView, Address, ProgramResult};
use solana_program_error::ProgramError;
use wincode::Deserialize as _;

use crate::state::Config;

wincode::pod_wrapper! {
    unsafe struct InstructionPodAddress(Address);
}

#[repr(C)]
#[derive(wincode::SchemaRead)]
pub struct ChangeAuthorityInstructionData {
    field: u8,
    #[wincode(with = "InstructionPodAddress")]
    new_authority: Address,
}

const IXN_DATA_LEN: usize = core::mem::size_of::<ChangeAuthorityInstructionData>();

#[repr(u8)]
enum AdminAuthorityField {
    Admin = 0,
    Override = 1,
    Status = 2,
}

impl AdminAuthorityField {
    fn from_u8(value: u8) -> Result<Self, ProgramError> {
        match value {
            0 => Ok(Self::Admin),
            1 => Ok(Self::Override),
            2 => Ok(Self::Status),
            _ => Err(ProgramError::InvalidInstructionData),
        }
    }
}

// Checks:
//   - admin is a signer
//   - config is owned by this program
//   - instruction data is exactly IXN_DATA_LEN bytes and deserializes successfully
//   - admin address matches config.admin_authority
//   returns the (field, new_authority) from instruction data
fn checks(
    program_id: &Address,
    admin: &AccountView,
    config: &AccountView,
    data: &[u8],
) -> Result<(u8, Address), ProgramError> {
    if !admin.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if !config.owned_by(program_id) {
        return Err(ProgramError::InvalidAccountOwner);
    }

    if data.len() != IXN_DATA_LEN {
        return Err(ProgramError::InvalidInstructionData);
    }

    let instruction_data = ChangeAuthorityInstructionData::deserialize(data)
        .map_err(|_| ProgramError::InvalidInstructionData)?;
    let field = instruction_data.field;
    let new_authority = instruction_data.new_authority;

    let config_data = config.try_borrow()?;
    let config_state = Config::load(&config_data)?;

    if admin.address() != &config_state.admin_authority {
        return Err(ProgramError::MissingRequiredSignature);
    }

    Ok((field, new_authority))
}

/// Accounts:
///   0. `[signer]`   admin (must match config.admin_authority)
///   1. `[writable]` config
///
/// Instruction data: `[field: u8, new_authority: Address]` (33 bytes)
///
/// Field values:
///   0 = admin_authority
///   1 = override_authority
///   2 = status_authority
pub fn process(program_id: &Address, accounts: &[AccountView], data: &[u8]) -> ProgramResult {
    let [admin, config] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    let (field, new_authority) = checks(program_id, admin, config, data)?;
    let field = AdminAuthorityField::from_u8(field)?;

    let mut config_data = config.try_borrow_mut()?;
    let config_state = Config::load_mut(&mut config_data)?;

    match field {
        AdminAuthorityField::Admin => config_state.admin_authority = new_authority,
        AdminAuthorityField::Override => config_state.override_authority = new_authority,
        AdminAuthorityField::Status => config_state.status_authority = new_authority,
    }
    Ok(())
}
