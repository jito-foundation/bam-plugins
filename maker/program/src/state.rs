use core::mem::size_of;

use pinocchio::Address;
use solana_program_error::ProgramError;
use wincode::{SchemaRead, SchemaWrite, ZeroCopy as _};

wincode::pod_wrapper! {
    unsafe struct PodAddress(Address);
}

pub const IXN_DISCRIMINATOR_LEN: usize = 1;
const EXPECTED_CONFIG_ACCOUNT_SIZE: usize = 1024;
const EXPECTED_PROGRAM_CONFIG_ACCOUNT_SIZE: usize = 4096;
pub const PDA_SEED_COUNT: usize = 2;
pub const PROGRAM_SIGNER_COUNT: usize = 32;
pub const MARKET_CONFIG_COUNT: usize = 32;
pub const MARKET_ID_PROGRAM_PREFIX_LEN: usize = 31;
pub const MARKET_ID_INDEX_LEN: usize = 1;

const CONFIG_USED_SIZE: usize = size_of::<Address>() * 3;
const PROGRAM_CONFIG_USED_SIZE: usize = size_of::<Address>() * 2
    + size_of::<MemCmp>()
    + size_of::<SignerConfig>() * PROGRAM_SIGNER_COUNT
    + size_of::<MarketConfig>() * MARKET_CONFIG_COUNT
    + size_of::<ProgramStatus>();

const CONFIG_PADDING_SIZE: usize = EXPECTED_CONFIG_ACCOUNT_SIZE - CONFIG_USED_SIZE;
const PROGRAM_CONFIG_PADDING_SIZE: usize =
    EXPECTED_PROGRAM_CONFIG_ACCOUNT_SIZE - PROGRAM_CONFIG_USED_SIZE;

const CONFIG_ACCOUNT_SIZE: usize = size_of::<ConfigState>();
const PROGRAM_CONFIG_ACCOUNT_SIZE: usize = size_of::<ProgramConfig>();
const _: () = assert!(
    EXPECTED_CONFIG_ACCOUNT_SIZE == CONFIG_ACCOUNT_SIZE,
    "ConfigState is not the expected size. If this is intentional, update EXPECTED_CONFIG_ACCOUNT_SIZE to match the new size and perform a proper migration of on-chain state."
);
const _: () = assert!(
    EXPECTED_PROGRAM_CONFIG_ACCOUNT_SIZE == PROGRAM_CONFIG_ACCOUNT_SIZE,
    "ProgramConfig is not the expected size. If this is intentional, update EXPECTED_PROGRAM_CONFIG_ACCOUNT_SIZE to match the new size and perform a proper migration of on-chain state."
);

#[repr(C)]
#[derive(SchemaWrite, SchemaRead)]
#[wincode(assert_zero_copy)]
pub struct ConfigState {
    #[wincode(with = "PodAddress")]
    pub admin_authority: Address,
    #[wincode(with = "PodAddress")]
    pub override_authority: Address,
    #[wincode(with = "PodAddress")]
    pub status_authority: Address,
    pub padding: [u8; CONFIG_PADDING_SIZE],
}

pub type Config = ConfigState;

impl ConfigState {
    pub fn load(data: &[u8]) -> Result<&Self, ProgramError> {
        Self::from_bytes(data).map_err(|_| ProgramError::InvalidAccountData)
    }

    pub fn load_mut(data: &mut [u8]) -> Result<&mut Self, ProgramError> {
        Self::from_bytes_mut(data).map_err(|_| ProgramError::InvalidAccountData)
    }
}

impl Default for ConfigState {
    fn default() -> Self {
        Self {
            admin_authority: Address::default(),
            override_authority: Address::default(),
            status_authority: Address::default(),
            padding: [0u8; CONFIG_PADDING_SIZE],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, SchemaWrite, SchemaRead)]
#[wincode(assert_zero_copy)]
pub struct MemCmp {
    pub offset: u16,
    pub length: u16,
}

impl Default for MemCmp {
    fn default() -> Self {
        Self {
            offset: 0,
            length: 0,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, SchemaWrite, SchemaRead)]
#[wincode(assert_zero_copy)]
pub struct MarketConfig {
    #[wincode(with = "PodAddress")]
    pub market_id: Address,
    #[wincode(with = "PodAddress")]
    pub writable_account: Address,
}

impl Default for MarketConfig {
    fn default() -> Self {
        Self {
            market_id: Address::default(),
            writable_account: Address::default(),
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, SchemaWrite, SchemaRead)]
#[wincode(assert_zero_copy)]
pub struct SignerConfig {
    #[wincode(with = "PodAddress")]
    pub signer: Address,
}

impl Default for SignerConfig {
    fn default() -> Self {
        Self {
            signer: Address::default(),
        }
    }
}

#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, SchemaWrite, SchemaRead)]
#[wincode(assert_zero_copy)]
pub struct ProgramStatus(u8);

#[allow(non_upper_case_globals)]
impl ProgramStatus {
    pub const Enrolled: Self = Self(0);
    pub const Active: Self = Self(1);
    pub const Unenroll: Self = Self(2);
    pub const OverrideUnenroll: Self = Self(3);

    pub const fn as_u8(self) -> u8 {
        self.0
    }
}

impl Default for ProgramStatus {
    fn default() -> Self {
        Self::Enrolled
    }
}

#[repr(C)]
#[derive(SchemaWrite, SchemaRead)]
#[wincode(assert_zero_copy)]
pub struct ProgramConfig {
    #[wincode(with = "PodAddress")]
    pub program_id: Address,
    #[wincode(with = "PodAddress")]
    pub delegate_authority: Address,
    pub seqno_instruction_data_offset: MemCmp,
    pub signer_configs: [SignerConfig; PROGRAM_SIGNER_COUNT],
    pub market_configs: [MarketConfig; MARKET_CONFIG_COUNT],
    pub status: ProgramStatus,
    pub padding: [u8; PROGRAM_CONFIG_PADDING_SIZE],
}

impl Default for ProgramConfig {
    fn default() -> Self {
        Self {
            program_id: Address::default(),
            delegate_authority: Address::default(),
            seqno_instruction_data_offset: MemCmp::default(),
            signer_configs: core::array::from_fn(|_| SignerConfig::default()),
            market_configs: core::array::from_fn(|_| MarketConfig::default()),
            status: ProgramStatus::default(),
            padding: [0u8; PROGRAM_CONFIG_PADDING_SIZE],
        }
    }
}

pub fn is_default_address(address: &Address) -> bool {
    address == &Address::default()
}

pub const CONFIG_SIZE: usize = size_of::<ConfigState>();
pub const PROGRAM_CONFIG_SIZE: usize = size_of::<ProgramConfig>();

impl ProgramConfig {
    pub fn load(data: &[u8]) -> Result<&Self, ProgramError> {
        Self::from_bytes(data).map_err(|_| ProgramError::InvalidAccountData)
    }

    pub fn load_mut(data: &mut [u8]) -> Result<&mut Self, ProgramError> {
        Self::from_bytes_mut(data).map_err(|_| ProgramError::InvalidAccountData)
    }
}
