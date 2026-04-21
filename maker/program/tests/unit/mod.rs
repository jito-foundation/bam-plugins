use bincode::serialize;
use mollusk_svm::{program::keyed_account_for_system_program, result::Check, Mollusk};
use solana_account::Account;
use solana_instruction::{error::InstructionError, AccountMeta, Instruction};
use solana_loader_v3_interface::state::UpgradeableLoaderState;
use solana_pubkey::Pubkey;

use crate::state::{CONFIG_SIZE, PROGRAM_CONFIG_SIZE};

/// Builds the two accounts required by `verify_upgrade_authority`:
///   - `program_account`: serialized `UpgradeableLoaderState::Program`
///   - `executable_data`: serialized `UpgradeableLoaderState::ProgramData`
fn make_upgrade_authority_accounts(
    authority: &Pubkey,
    executable_data_key: &Pubkey,
) -> (Account, Account) {
    let program_account_data = serialize(&UpgradeableLoaderState::Program {
        programdata_address: solana_pubkey::Pubkey::new_from_array(executable_data_key.to_bytes()),
    })
    .unwrap();

    let exec_data_bytes = serialize(&UpgradeableLoaderState::ProgramData {
        slot: 0,
        upgrade_authority_address: Some(solana_pubkey::Pubkey::new_from_array(
            authority.to_bytes(),
        )),
    })
    .unwrap();

    let program_account = Account {
        lamports: 1_000_000,
        data: program_account_data,
        owner: solana_sdk_ids::system_program::id(),
        executable: false,
        rent_epoch: 0,
    };
    let executable_data_account = Account {
        lamports: 1_000_000,
        data: exec_data_bytes,
        owner: solana_sdk_ids::system_program::id(),
        executable: false,
        rent_epoch: 0,
    };
    (program_account, executable_data_account)
}

fn assert_program_error(
    result: mollusk_svm::result::InstructionResult,
    expected: InstructionError,
) {
    assert_eq!(result.raw_result, Err(expected));
}

fn serialize_bytes<T>(value: &T) -> Vec<u8>
where
    T: wincode::Serialize<Src = T> + ?Sized,
{
    <T as wincode::Serialize>::serialize(value).unwrap()
}

fn raw_program_config_bytes(
    target_program_key: Pubkey,
    status: crate::state::ProgramStatus,
    delegate_authority: Option<Pubkey>,
) -> Vec<u8> {
    let mut data = vec![0u8; PROGRAM_CONFIG_SIZE];
    let program_config = crate::state::ProgramConfig::load_mut(&mut data).unwrap();
    program_config.program_id = target_program_key.to_bytes().into();
    program_config.status = status;
    if let Some(delegate_authority) = delegate_authority {
        program_config.delegate_authority = delegate_authority.to_bytes().into();
    }
    data
}

fn derived_market_id(target_program_key: Pubkey, index: u8) -> [u8; 32] {
    let mut market_id = [0u8; 32];
    market_id[..31].copy_from_slice(&target_program_key.to_bytes()[..31]);
    market_id[31] = index;
    market_id
}

fn update_market_instruction_data(index: u8, writable_account: [u8; 32]) -> Vec<u8> {
    [&[2u8, index][..], &writable_account].concat()
}

fn update_program_signer_instruction_data(index: u8, signer: Pubkey) -> Vec<u8> {
    [&[8u8, index][..], signer.as_ref()].concat()
}

fn update_program_memcmp_instruction_data(offset: u16, length: u16) -> Vec<u8> {
    let mut data = vec![9u8];
    data.extend_from_slice(&offset.to_le_bytes());
    data.extend_from_slice(&length.to_le_bytes());
    data
}

#[test]
fn test_init_config() {
    let program_id = Pubkey::new_unique();
    let mollusk = Mollusk::new(&program_id, "target/deploy/maker_registry");
    let rent_sysvar = mollusk.sysvars.keyed_account_for_rent_sysvar();

    let (config_pda, bump) = Pubkey::find_program_address(&[b"config"], &program_id);

    let payer = Pubkey::new_unique();
    let payer_account = Account::new(10_000_000_000, 0, &solana_sdk_ids::system_program::id());

    // Discriminant 0 = init_config, followed by the bump seed.
    let instruction_data = [0u8, bump];

    let instruction = Instruction::new_with_bytes(
        program_id,
        &instruction_data,
        vec![
            AccountMeta::new(payer, true),
            AccountMeta::new(config_pda, false),
            AccountMeta::new_readonly(solana_sdk_ids::system_program::id(), false),
            AccountMeta::new_readonly(rent_sysvar.0, false),
        ],
    );

    let accounts = vec![
        (payer, payer_account),
        (config_pda, Account::default()),
        keyed_account_for_system_program(),
        rent_sysvar,
    ];

    mollusk.process_and_validate_instruction(
        &instruction,
        &accounts,
        &[
            Check::success(),
            Check::account(&config_pda)
                .space(CONFIG_SIZE)
                .rent_exempt()
                .build(),
        ],
    );
}

#[test]
fn test_override_unenroll() {
    use crate::state::{Config, ProgramConfig, ProgramStatus};

    let program_id = Pubkey::new_unique();
    let mollusk = Mollusk::new(&program_id, "target/deploy/maker_registry");

    let authority = Pubkey::new_unique();
    let config_key = Pubkey::new_unique();
    let program_config_key = Pubkey::new_unique();

    let mut config_struct = Config::default();
    config_struct.override_authority = authority.to_bytes().into();
    let config_data = serialize_bytes(&config_struct);

    let mut program_config_struct = ProgramConfig::default();
    program_config_struct.status = ProgramStatus::Enrolled;
    let program_config_data = serialize_bytes(&program_config_struct);

    let config_account = Account {
        lamports: 10_000_000,
        data: config_data,
        owner: program_id,
        executable: false,
        rent_epoch: 0,
    };
    let program_config_account = Account {
        lamports: 10_000_000,
        data: program_config_data,
        owner: program_id,
        executable: false,
        rent_epoch: 0,
    };
    let authority_account = Account::new(1_000_000, 0, &solana_sdk_ids::system_program::id());

    // Discriminant 5 = override_unenroll, no additional instruction data
    let instruction_data = [5u8];

    let instruction = Instruction::new_with_bytes(
        program_id,
        &instruction_data,
        vec![
            AccountMeta::new_readonly(authority, true),
            AccountMeta::new_readonly(config_key, false),
            AccountMeta::new(program_config_key, false),
        ],
    );

    let accounts = vec![
        (authority, authority_account),
        (config_key, config_account),
        (program_config_key, program_config_account),
    ];

    let result =
        mollusk.process_and_validate_instruction(&instruction, &accounts, &[Check::success()]);

    let (_, resulting_program_config) = result
        .resulting_accounts
        .iter()
        .find(|(k, _)| k == &program_config_key)
        .expect("program_config account not found in result");

    let resulting_state = ProgramConfig::load(&resulting_program_config.data).unwrap();
    assert_eq!(resulting_state.status, ProgramStatus::OverrideUnenroll);
}

#[test]
fn test_init_program_config() {
    use crate::state::{ProgramConfig, ProgramStatus};

    let program_id = Pubkey::new_unique();
    let mollusk = Mollusk::new(&program_id, "target/deploy/maker_registry");
    let rent_sysvar = mollusk.sysvars.keyed_account_for_rent_sysvar();

    let authority = Pubkey::new_unique();
    let config_key = Pubkey::new_unique();
    let target_program_key = Pubkey::new_unique();
    let executable_data_key = Pubkey::new_unique();

    let (program_account, executable_data_account) =
        make_upgrade_authority_accounts(&authority, &executable_data_key);

    let (program_config_pda, bump) =
        Pubkey::find_program_address(&[&target_program_key.to_bytes()], &program_id);

    let authority_account = Account::new(10_000_000_000, 0, &solana_sdk_ids::system_program::id());
    let config_account = Account {
        lamports: 10_000_000,
        data: vec![0u8; CONFIG_SIZE],
        owner: program_id,
        executable: false,
        rent_epoch: 0,
    };

    // Discriminant 1 = init_program_config, followed by target_program_id (32 bytes) and bump (1 byte)
    let mut instruction_data = vec![1u8];
    instruction_data.extend_from_slice(&target_program_key.to_bytes());
    instruction_data.push(bump);

    let instruction = Instruction::new_with_bytes(
        program_id,
        &instruction_data,
        vec![
            AccountMeta::new(authority, true),
            AccountMeta::new(program_config_pda, false),
            AccountMeta::new_readonly(config_key, false),
            AccountMeta::new_readonly(target_program_key, false),
            AccountMeta::new_readonly(executable_data_key, false),
            AccountMeta::new_readonly(solana_sdk_ids::system_program::id(), false),
            AccountMeta::new_readonly(rent_sysvar.0, false),
        ],
    );

    let accounts = vec![
        (authority, authority_account),
        (program_config_pda, Account::default()),
        (config_key, config_account),
        (target_program_key, program_account),
        (executable_data_key, executable_data_account),
        keyed_account_for_system_program(),
        rent_sysvar,
    ];

    let result = mollusk.process_and_validate_instruction(
        &instruction,
        &accounts,
        &[
            Check::success(),
            Check::account(&program_config_pda)
                .space(PROGRAM_CONFIG_SIZE)
                .rent_exempt()
                .build(),
        ],
    );

    let (_, resulting_program_config) = result
        .resulting_accounts
        .iter()
        .find(|(k, _)| k == &program_config_pda)
        .expect("program_config account not found in result");

    let resulting_state = ProgramConfig::load(&resulting_program_config.data).unwrap();
    assert_eq!(
        resulting_state.program_id,
        target_program_key.to_bytes().into()
    );
    assert_eq!(resulting_state.status, ProgramStatus::Enrolled);
}

#[test]
fn test_update_market_config() {
    use crate::state::{ProgramConfig, ProgramStatus};

    let program_id = Pubkey::new_unique();
    let mollusk = Mollusk::new(&program_id, "target/deploy/maker_registry");

    let authority = Pubkey::new_unique();
    let config_key = Pubkey::new_unique();
    let target_program_key = Pubkey::new_unique();
    let executable_data_key = Pubkey::new_unique();
    let program_config_key = Pubkey::new_unique();

    let (program_account, executable_data_account) =
        make_upgrade_authority_accounts(&authority, &executable_data_key);

    let program_config_data =
        raw_program_config_bytes(target_program_key, ProgramStatus::Enrolled, None);

    let program_config_account = Account {
        lamports: 10_000_000,
        data: program_config_data,
        owner: program_id,
        executable: false,
        rent_epoch: 0,
    };
    let authority_account = Account::new(1_000_000, 0, &solana_sdk_ids::system_program::id());
    let config_account = Account {
        lamports: 10_000_000,
        data: vec![0u8; CONFIG_SIZE],
        owner: program_id,
        executable: false,
        rent_epoch: 0,
    };

    let index = 0u8;
    let instruction_data = update_market_instruction_data(index, [9u8; 32]);

    let instruction = Instruction::new_with_bytes(
        program_id,
        &instruction_data,
        vec![
            AccountMeta::new_readonly(authority, true),
            AccountMeta::new(program_config_key, false),
            AccountMeta::new_readonly(config_key, false),
            AccountMeta::new_readonly(target_program_key, false),
            AccountMeta::new_readonly(executable_data_key, false),
        ],
    );

    let accounts = vec![
        (authority, authority_account),
        (program_config_key, program_config_account),
        (config_key, config_account),
        (target_program_key, program_account),
        (executable_data_key, executable_data_account),
    ];

    let result =
        mollusk.process_and_validate_instruction(&instruction, &accounts, &[Check::success()]);

    let (_, resulting_program_config) = result
        .resulting_accounts
        .iter()
        .find(|(k, _)| k == &program_config_key)
        .expect("program_config account not found in result");

    let resulting_state = ProgramConfig::load(&resulting_program_config.data).unwrap();
    assert_eq!(
        resulting_state.market_configs[0].market_id,
        derived_market_id(target_program_key, index).into()
    );
    assert_eq!(
        resulting_state.market_configs[0].writable_account,
        [9u8; 32].into()
    );
    assert_eq!(resulting_state.signer_configs[0].signer, [0u8; 32].into());
    assert_eq!(resulting_state.seqno_instruction_data_offset.offset, 0);
    assert_eq!(resulting_state.seqno_instruction_data_offset.length, 0);
}

#[test]
fn test_upgrade_authority_unenroll() {
    use crate::state::{ProgramConfig, ProgramStatus};

    let program_id = Pubkey::new_unique();
    let mollusk = Mollusk::new(&program_id, "target/deploy/maker_registry");

    let authority = Pubkey::new_unique();
    let config_key = Pubkey::new_unique();
    let target_program_key = Pubkey::new_unique();
    let executable_data_key = Pubkey::new_unique();
    let program_config_key = Pubkey::new_unique();

    let (program_account, executable_data_account) =
        make_upgrade_authority_accounts(&authority, &executable_data_key);

    let mut program_config_struct = ProgramConfig::default();
    program_config_struct.program_id = target_program_key.to_bytes().into();
    program_config_struct.status = ProgramStatus::Enrolled;
    let program_config_data = serialize_bytes(&program_config_struct);

    let program_config_account = Account {
        lamports: 10_000_000,
        data: program_config_data,
        owner: program_id,
        executable: false,
        rent_epoch: 0,
    };
    let authority_account = Account::new(1_000_000, 0, &solana_sdk_ids::system_program::id());
    let config_account = Account {
        lamports: 10_000_000,
        data: vec![0u8; CONFIG_SIZE],
        owner: program_id,
        executable: false,
        rent_epoch: 0,
    };

    // Discriminant 3 = upgrade_authority_unenroll, no additional instruction data
    let instruction_data = [3u8];

    let instruction = Instruction::new_with_bytes(
        program_id,
        &instruction_data,
        vec![
            AccountMeta::new_readonly(authority, true),
            AccountMeta::new_readonly(config_key, false),
            AccountMeta::new_readonly(target_program_key, false),
            AccountMeta::new_readonly(executable_data_key, false),
            AccountMeta::new(program_config_key, false),
        ],
    );

    let accounts = vec![
        (authority, authority_account),
        (config_key, config_account),
        (target_program_key, program_account),
        (executable_data_key, executable_data_account),
        (program_config_key, program_config_account),
    ];

    let result =
        mollusk.process_and_validate_instruction(&instruction, &accounts, &[Check::success()]);

    let (_, resulting_program_config) = result
        .resulting_accounts
        .iter()
        .find(|(k, _)| k == &program_config_key)
        .expect("program_config account not found in result");

    let resulting_state = ProgramConfig::load(&resulting_program_config.data).unwrap();
    assert_eq!(resulting_state.status, ProgramStatus::Unenroll);
}

#[test]
fn test_assign_delegate_authority_as_override_authority() {
    use crate::state::{Config, ProgramConfig};

    let program_id = Pubkey::new_unique();
    let mollusk = Mollusk::new(&program_id, "target/deploy/maker_registry");

    let override_authority = Pubkey::new_unique();
    let delegate_authority = Pubkey::new_unique();
    let target_program_key = Pubkey::new_unique();
    let executable_data_key = Pubkey::new_unique();
    let config_key = Pubkey::new_unique();
    let program_config_key = Pubkey::new_unique();

    let mut config_struct = Config::default();
    config_struct.override_authority = override_authority.to_bytes().into();
    let config_data = serialize_bytes(&config_struct);

    let mut program_config_struct = ProgramConfig::default();
    program_config_struct.program_id = target_program_key.to_bytes().into();
    let program_config_data = serialize_bytes(&program_config_struct);

    let config_account = Account {
        lamports: 10_000_000,
        data: config_data,
        owner: program_id,
        executable: false,
        rent_epoch: 0,
    };
    let program_config_account = Account {
        lamports: 10_000_000,
        data: program_config_data,
        owner: program_id,
        executable: false,
        rent_epoch: 0,
    };
    let override_authority_account =
        Account::new(1_000_000, 0, &solana_sdk_ids::system_program::id());
    let (program_account, executable_data_account) =
        make_upgrade_authority_accounts(&Pubkey::new_unique(), &executable_data_key);

    let instruction = Instruction::new_with_bytes(
        program_id,
        &[&[7u8], delegate_authority.as_ref()].concat(),
        vec![
            AccountMeta::new_readonly(override_authority, true),
            AccountMeta::new_readonly(config_key, false),
            AccountMeta::new(program_config_key, false),
            AccountMeta::new_readonly(target_program_key, false),
            AccountMeta::new_readonly(executable_data_key, false),
        ],
    );

    let accounts = vec![
        (override_authority, override_authority_account),
        (config_key, config_account),
        (program_config_key, program_config_account),
        (target_program_key, program_account),
        (executable_data_key, executable_data_account),
    ];

    let result =
        mollusk.process_and_validate_instruction(&instruction, &accounts, &[Check::success()]);

    let (_, resulting_program_config) = result
        .resulting_accounts
        .iter()
        .find(|(k, _)| k == &program_config_key)
        .expect("program_config account not found in result");

    let resulting_state = ProgramConfig::load(&resulting_program_config.data).unwrap();
    assert_eq!(
        resulting_state.delegate_authority,
        delegate_authority.to_bytes().into()
    );
}

#[test]
fn test_update_market_config_as_delegate_authority() {
    use crate::state::{ProgramConfig, ProgramStatus};

    let program_id = Pubkey::new_unique();
    let mollusk = Mollusk::new(&program_id, "target/deploy/maker_registry");

    let delegate_authority = Pubkey::new_unique();
    let config_key = Pubkey::new_unique();
    let target_program_key = Pubkey::new_unique();
    let executable_data_key = Pubkey::new_unique();
    let program_config_key = Pubkey::new_unique();

    let (program_account, executable_data_account) =
        make_upgrade_authority_accounts(&Pubkey::new_unique(), &executable_data_key);

    let authority_account = Account::new(10_000_000_000, 0, &solana_sdk_ids::system_program::id());
    let config_account = Account {
        lamports: 10_000_000,
        data: vec![0u8; CONFIG_SIZE],
        owner: program_id,
        executable: false,
        rent_epoch: 0,
    };

    let program_config_data = raw_program_config_bytes(
        target_program_key,
        ProgramStatus::Enrolled,
        Some(delegate_authority),
    );

    let program_config_account = Account {
        lamports: 10_000_000,
        data: program_config_data,
        owner: program_id,
        executable: false,
        rent_epoch: 0,
    };

    let index = 0u8;
    let instruction_data = update_market_instruction_data(index, [9u8; 32]);
    let instruction = Instruction::new_with_bytes(
        program_id,
        &instruction_data,
        vec![
            AccountMeta::new_readonly(delegate_authority, true),
            AccountMeta::new(program_config_key, false),
            AccountMeta::new_readonly(config_key, false),
            AccountMeta::new_readonly(target_program_key, false),
            AccountMeta::new_readonly(executable_data_key, false),
        ],
    );

    let accounts = vec![
        (delegate_authority, authority_account),
        (program_config_key, program_config_account),
        (config_key, config_account),
        (target_program_key, program_account),
        (executable_data_key, executable_data_account),
    ];

    let result =
        mollusk.process_and_validate_instruction(&instruction, &accounts, &[Check::success()]);

    let (_, resulting_program_config) = result
        .resulting_accounts
        .iter()
        .find(|(k, _)| k == &program_config_key)
        .expect("program_config account not found in result");

    let resulting_state = ProgramConfig::load(&resulting_program_config.data).unwrap();
    assert_eq!(
        resulting_state.market_configs[0].market_id,
        derived_market_id(target_program_key, index).into()
    );
    assert_eq!(
        resulting_state.market_configs[0].writable_account,
        [9u8; 32].into()
    );
    assert_eq!(resulting_state.signer_configs[0].signer, [0u8; 32].into());
}

#[test]
fn test_upgrade_authority_unenroll_as_delegate_authority() {
    use crate::state::{ProgramConfig, ProgramStatus};

    let program_id = Pubkey::new_unique();
    let mollusk = Mollusk::new(&program_id, "target/deploy/maker_registry");

    let delegate_authority = Pubkey::new_unique();
    let config_key = Pubkey::new_unique();
    let target_program_key = Pubkey::new_unique();
    let executable_data_key = Pubkey::new_unique();
    let program_config_key = Pubkey::new_unique();

    let (program_account, executable_data_account) =
        make_upgrade_authority_accounts(&Pubkey::new_unique(), &executable_data_key);

    let authority_account = Account::new(10_000_000_000, 0, &solana_sdk_ids::system_program::id());
    let config_account = Account {
        lamports: 10_000_000,
        data: vec![0u8; CONFIG_SIZE],
        owner: program_id,
        executable: false,
        rent_epoch: 0,
    };

    let mut program_config_struct = ProgramConfig::default();
    program_config_struct.program_id = target_program_key.to_bytes().into();
    program_config_struct.delegate_authority = delegate_authority.to_bytes().into();
    program_config_struct.status = ProgramStatus::Active;
    let program_config_data = serialize_bytes(&program_config_struct);

    let program_config_account = Account {
        lamports: 10_000_000,
        data: program_config_data,
        owner: program_id,
        executable: false,
        rent_epoch: 0,
    };

    let instruction = Instruction::new_with_bytes(
        program_id,
        &[3u8],
        vec![
            AccountMeta::new_readonly(delegate_authority, true),
            AccountMeta::new_readonly(config_key, false),
            AccountMeta::new_readonly(target_program_key, false),
            AccountMeta::new_readonly(executable_data_key, false),
            AccountMeta::new(program_config_key, false),
        ],
    );

    let accounts = vec![
        (delegate_authority, authority_account),
        (config_key, config_account),
        (target_program_key, program_account),
        (executable_data_key, executable_data_account),
        (program_config_key, program_config_account),
    ];

    let result =
        mollusk.process_and_validate_instruction(&instruction, &accounts, &[Check::success()]);

    let (_, resulting_program_config) = result
        .resulting_accounts
        .iter()
        .find(|(k, _)| k == &program_config_key)
        .expect("program_config account not found in result");

    let resulting_state = ProgramConfig::load(&resulting_program_config.data).unwrap();
    assert_eq!(resulting_state.status, ProgramStatus::Unenroll);
}

#[test]
fn test_init_program_config_as_override_authority() {
    use crate::state::{Config, ProgramConfig, ProgramStatus};

    let program_id = Pubkey::new_unique();
    let mollusk = Mollusk::new(&program_id, "target/deploy/maker_registry");
    let rent_sysvar = mollusk.sysvars.keyed_account_for_rent_sysvar();

    let override_authority = Pubkey::new_unique();
    let config_key = Pubkey::new_unique();
    let target_program_key = Pubkey::new_unique();
    let executable_data_key = Pubkey::new_unique();

    let mut config_struct = Config::default();
    config_struct.override_authority = override_authority.to_bytes().into();
    let config_data = serialize_bytes(&config_struct);

    let (program_account, executable_data_account) =
        make_upgrade_authority_accounts(&Pubkey::new_unique(), &executable_data_key);

    let (program_config_pda, bump) =
        Pubkey::find_program_address(&[&target_program_key.to_bytes()], &program_id);

    let authority_account = Account::new(10_000_000_000, 0, &solana_sdk_ids::system_program::id());
    let config_account = Account {
        lamports: 10_000_000,
        data: config_data,
        owner: program_id,
        executable: false,
        rent_epoch: 0,
    };

    let mut instruction_data = vec![1u8];
    instruction_data.extend_from_slice(&target_program_key.to_bytes());
    instruction_data.push(bump);

    let instruction = Instruction::new_with_bytes(
        program_id,
        &instruction_data,
        vec![
            AccountMeta::new(override_authority, true),
            AccountMeta::new(program_config_pda, false),
            AccountMeta::new_readonly(config_key, false),
            AccountMeta::new_readonly(target_program_key, false),
            AccountMeta::new_readonly(executable_data_key, false),
            AccountMeta::new_readonly(solana_sdk_ids::system_program::id(), false),
            AccountMeta::new_readonly(rent_sysvar.0, false),
        ],
    );

    let accounts = vec![
        (override_authority, authority_account),
        (program_config_pda, Account::default()),
        (config_key, config_account),
        (target_program_key, program_account),
        (executable_data_key, executable_data_account),
        keyed_account_for_system_program(),
        rent_sysvar,
    ];

    let result =
        mollusk.process_and_validate_instruction(&instruction, &accounts, &[Check::success()]);

    let (_, resulting_program_config) = result
        .resulting_accounts
        .iter()
        .find(|(k, _)| k == &program_config_pda)
        .expect("program_config account not found in result");

    let resulting_state = ProgramConfig::load(&resulting_program_config.data).unwrap();
    assert_eq!(
        resulting_state.program_id,
        target_program_key.to_bytes().into()
    );
    assert_eq!(resulting_state.status, ProgramStatus::Enrolled);
}

#[test]
fn test_update_market_config_as_override_authority() {
    use crate::state::{Config, ProgramConfig, ProgramStatus};

    let program_id = Pubkey::new_unique();
    let mollusk = Mollusk::new(&program_id, "target/deploy/maker_registry");

    let override_authority = Pubkey::new_unique();
    let config_key = Pubkey::new_unique();
    let target_program_key = Pubkey::new_unique();
    let executable_data_key = Pubkey::new_unique();
    let program_config_key = Pubkey::new_unique();

    let mut config_struct = Config::default();
    config_struct.override_authority = override_authority.to_bytes().into();
    let config_data = serialize_bytes(&config_struct);

    let (program_account, executable_data_account) =
        make_upgrade_authority_accounts(&Pubkey::new_unique(), &executable_data_key);

    let authority_account = Account::new(10_000_000_000, 0, &solana_sdk_ids::system_program::id());
    let config_account = Account {
        lamports: 10_000_000,
        data: config_data,
        owner: program_id,
        executable: false,
        rent_epoch: 0,
    };

    let program_config_data =
        raw_program_config_bytes(target_program_key, ProgramStatus::Enrolled, None);

    let program_config_account = Account {
        lamports: 10_000_000,
        data: program_config_data,
        owner: program_id,
        executable: false,
        rent_epoch: 0,
    };

    let index = 0u8;
    let instruction_data = update_market_instruction_data(index, [9u8; 32]);
    let instruction = Instruction::new_with_bytes(
        program_id,
        &instruction_data,
        vec![
            AccountMeta::new_readonly(override_authority, true),
            AccountMeta::new(program_config_key, false),
            AccountMeta::new_readonly(config_key, false),
            AccountMeta::new_readonly(target_program_key, false),
            AccountMeta::new_readonly(executable_data_key, false),
        ],
    );

    let accounts = vec![
        (override_authority, authority_account),
        (program_config_key, program_config_account),
        (config_key, config_account),
        (target_program_key, program_account),
        (executable_data_key, executable_data_account),
    ];

    let result =
        mollusk.process_and_validate_instruction(&instruction, &accounts, &[Check::success()]);

    let (_, resulting_program_config) = result
        .resulting_accounts
        .iter()
        .find(|(k, _)| k == &program_config_key)
        .expect("program_config account not found in result");

    let resulting_state = ProgramConfig::load(&resulting_program_config.data).unwrap();
    assert_eq!(
        resulting_state.market_configs[0].market_id,
        derived_market_id(target_program_key, index).into()
    );
    assert_eq!(
        resulting_state.market_configs[0].writable_account,
        [9u8; 32].into()
    );
    assert_eq!(resulting_state.signer_configs[0].signer, [0u8; 32].into());
}

#[test]
fn test_upgrade_authority_unenroll_as_override_authority() {
    use crate::state::{Config, ProgramConfig, ProgramStatus};

    let program_id = Pubkey::new_unique();
    let mollusk = Mollusk::new(&program_id, "target/deploy/maker_registry");

    let override_authority = Pubkey::new_unique();
    let config_key = Pubkey::new_unique();
    let target_program_key = Pubkey::new_unique();
    let executable_data_key = Pubkey::new_unique();
    let program_config_key = Pubkey::new_unique();

    let mut config_struct = Config::default();
    config_struct.override_authority = override_authority.to_bytes().into();
    let config_data = serialize_bytes(&config_struct);

    let (program_account, executable_data_account) =
        make_upgrade_authority_accounts(&Pubkey::new_unique(), &executable_data_key);

    let authority_account = Account::new(10_000_000_000, 0, &solana_sdk_ids::system_program::id());
    let config_account = Account {
        lamports: 10_000_000,
        data: config_data,
        owner: program_id,
        executable: false,
        rent_epoch: 0,
    };

    let mut program_config_struct = ProgramConfig::default();
    program_config_struct.program_id = target_program_key.to_bytes().into();
    program_config_struct.status = ProgramStatus::Active;
    let program_config_data = serialize_bytes(&program_config_struct);

    let program_config_account = Account {
        lamports: 10_000_000,
        data: program_config_data,
        owner: program_id,
        executable: false,
        rent_epoch: 0,
    };

    let instruction = Instruction::new_with_bytes(
        program_id,
        &[3u8],
        vec![
            AccountMeta::new_readonly(override_authority, true),
            AccountMeta::new_readonly(config_key, false),
            AccountMeta::new_readonly(target_program_key, false),
            AccountMeta::new_readonly(executable_data_key, false),
            AccountMeta::new(program_config_key, false),
        ],
    );

    let accounts = vec![
        (override_authority, authority_account),
        (config_key, config_account),
        (target_program_key, program_account),
        (executable_data_key, executable_data_account),
        (program_config_key, program_config_account),
    ];

    let result =
        mollusk.process_and_validate_instruction(&instruction, &accounts, &[Check::success()]);

    let (_, resulting_program_config) = result
        .resulting_accounts
        .iter()
        .find(|(k, _)| k == &program_config_key)
        .expect("program_config account not found in result");

    let resulting_state = ProgramConfig::load(&resulting_program_config.data).unwrap();
    assert_eq!(resulting_state.status, ProgramStatus::Unenroll);
}

#[test]
fn test_activate_as_status_authority() {
    use crate::state::{Config, ProgramConfig, ProgramStatus};

    let program_id = Pubkey::new_unique();
    let mollusk = Mollusk::new(&program_id, "target/deploy/maker_registry");

    let status_authority = Pubkey::new_unique();
    let config_key = Pubkey::new_unique();
    let program_config_key = Pubkey::new_unique();

    let mut config_struct = Config::default();
    config_struct.status_authority = status_authority.to_bytes().into();
    let config_data = serialize_bytes(&config_struct);

    let mut program_config_struct = ProgramConfig::default();
    program_config_struct.status = ProgramStatus::Enrolled;
    let program_config_data = serialize_bytes(&program_config_struct);

    let config_account = Account {
        lamports: 10_000_000,
        data: config_data,
        owner: program_id,
        executable: false,
        rent_epoch: 0,
    };
    let program_config_account = Account {
        lamports: 10_000_000,
        data: program_config_data,
        owner: program_id,
        executable: false,
        rent_epoch: 0,
    };
    let authority_account = Account::new(1_000_000, 0, &solana_sdk_ids::system_program::id());

    let instruction = Instruction::new_with_bytes(
        program_id,
        &[4u8],
        vec![
            AccountMeta::new_readonly(status_authority, true),
            AccountMeta::new_readonly(config_key, false),
            AccountMeta::new(program_config_key, false),
        ],
    );

    let accounts = vec![
        (status_authority, authority_account),
        (config_key, config_account),
        (program_config_key, program_config_account),
    ];

    let result =
        mollusk.process_and_validate_instruction(&instruction, &accounts, &[Check::success()]);

    let (_, resulting_program_config) = result
        .resulting_accounts
        .iter()
        .find(|(k, _)| k == &program_config_key)
        .expect("program_config account not found in result");

    let resulting_state = ProgramConfig::load(&resulting_program_config.data).unwrap();
    assert_eq!(resulting_state.status, ProgramStatus::Active);
}

#[test]
fn test_init_program_config_rejects_unauthorized_authority() {
    let program_id = Pubkey::new_unique();
    let mollusk = Mollusk::new(&program_id, "target/deploy/maker_registry");
    let rent_sysvar = mollusk.sysvars.keyed_account_for_rent_sysvar();

    let unauthorized_authority = Pubkey::new_unique();
    let upgrade_authority = Pubkey::new_unique();
    let config_key = Pubkey::new_unique();
    let target_program_key = Pubkey::new_unique();
    let executable_data_key = Pubkey::new_unique();

    let (program_account, executable_data_account) =
        make_upgrade_authority_accounts(&upgrade_authority, &executable_data_key);

    let (program_config_pda, bump) =
        Pubkey::find_program_address(&[&target_program_key.to_bytes()], &program_id);

    let authority_account = Account::new(10_000_000_000, 0, &solana_sdk_ids::system_program::id());
    let config_account = Account {
        lamports: 10_000_000,
        data: vec![0u8; CONFIG_SIZE],
        owner: program_id,
        executable: false,
        rent_epoch: 0,
    };

    let mut instruction_data = vec![1u8];
    instruction_data.extend_from_slice(&target_program_key.to_bytes());
    instruction_data.push(bump);

    let instruction = Instruction::new_with_bytes(
        program_id,
        &instruction_data,
        vec![
            AccountMeta::new(unauthorized_authority, true),
            AccountMeta::new(program_config_pda, false),
            AccountMeta::new_readonly(config_key, false),
            AccountMeta::new_readonly(target_program_key, false),
            AccountMeta::new_readonly(executable_data_key, false),
            AccountMeta::new_readonly(solana_sdk_ids::system_program::id(), false),
            AccountMeta::new_readonly(rent_sysvar.0, false),
        ],
    );

    let accounts = vec![
        (unauthorized_authority, authority_account),
        (program_config_pda, Account::default()),
        (config_key, config_account),
        (target_program_key, program_account),
        (executable_data_key, executable_data_account),
        keyed_account_for_system_program(),
        rent_sysvar,
    ];

    let result = mollusk.process_instruction(&instruction, &accounts);
    assert_program_error(result, InstructionError::MissingRequiredSignature);
}

#[test]
fn test_update_market_config_rejects_unauthorized_authority() {
    use crate::state::ProgramStatus;
    let program_id = Pubkey::new_unique();
    let mollusk = Mollusk::new(&program_id, "target/deploy/maker_registry");

    let unauthorized_authority = Pubkey::new_unique();
    let upgrade_authority = Pubkey::new_unique();
    let config_key = Pubkey::new_unique();
    let target_program_key = Pubkey::new_unique();
    let executable_data_key = Pubkey::new_unique();
    let program_config_key = Pubkey::new_unique();

    let (program_account, executable_data_account) =
        make_upgrade_authority_accounts(&upgrade_authority, &executable_data_key);

    let program_config_data =
        raw_program_config_bytes(target_program_key, ProgramStatus::Enrolled, None);

    let program_config_account = Account {
        lamports: 10_000_000,
        data: program_config_data,
        owner: program_id,
        executable: false,
        rent_epoch: 0,
    };
    let authority_account = Account::new(10_000_000_000, 0, &solana_sdk_ids::system_program::id());
    let config_account = Account {
        lamports: 10_000_000,
        data: vec![0u8; CONFIG_SIZE],
        owner: program_id,
        executable: false,
        rent_epoch: 0,
    };

    let instruction_data = update_market_instruction_data(0, [9u8; 32]);
    let instruction = Instruction::new_with_bytes(
        program_id,
        &instruction_data,
        vec![
            AccountMeta::new_readonly(unauthorized_authority, true),
            AccountMeta::new(program_config_key, false),
            AccountMeta::new_readonly(config_key, false),
            AccountMeta::new_readonly(target_program_key, false),
            AccountMeta::new_readonly(executable_data_key, false),
        ],
    );

    let accounts = vec![
        (unauthorized_authority, authority_account),
        (program_config_key, program_config_account),
        (config_key, config_account),
        (target_program_key, program_account),
        (executable_data_key, executable_data_account),
    ];

    let result = mollusk.process_instruction(&instruction, &accounts);
    assert_program_error(result, InstructionError::MissingRequiredSignature);
}

#[test]
fn test_update_program_signer() {
    use crate::state::{ProgramConfig, ProgramStatus};

    let program_id = Pubkey::new_unique();
    let mollusk = Mollusk::new(&program_id, "target/deploy/maker_registry");

    let authority = Pubkey::new_unique();
    let config_key = Pubkey::new_unique();
    let target_program_key = Pubkey::new_unique();
    let executable_data_key = Pubkey::new_unique();
    let program_config_key = Pubkey::new_unique();
    let new_signer = Pubkey::new_unique();

    let (program_account, executable_data_account) =
        make_upgrade_authority_accounts(&authority, &executable_data_key);

    let program_config_data =
        raw_program_config_bytes(target_program_key, ProgramStatus::Enrolled, None);

    let program_config_account = Account {
        lamports: 10_000_000,
        data: program_config_data,
        owner: program_id,
        executable: false,
        rent_epoch: 0,
    };
    let authority_account = Account::new(1_000_000, 0, &solana_sdk_ids::system_program::id());
    let config_account = Account {
        lamports: 10_000_000,
        data: vec![0u8; CONFIG_SIZE],
        owner: program_id,
        executable: false,
        rent_epoch: 0,
    };

    let instruction_data = update_program_signer_instruction_data(0, new_signer);
    let instruction = Instruction::new_with_bytes(
        program_id,
        &instruction_data,
        vec![
            AccountMeta::new_readonly(authority, true),
            AccountMeta::new(program_config_key, false),
            AccountMeta::new_readonly(config_key, false),
            AccountMeta::new_readonly(target_program_key, false),
            AccountMeta::new_readonly(executable_data_key, false),
        ],
    );

    let accounts = vec![
        (authority, authority_account),
        (program_config_key, program_config_account),
        (config_key, config_account),
        (target_program_key, program_account),
        (executable_data_key, executable_data_account),
    ];

    let result =
        mollusk.process_and_validate_instruction(&instruction, &accounts, &[Check::success()]);

    let (_, resulting_program_config) = result
        .resulting_accounts
        .iter()
        .find(|(k, _)| k == &program_config_key)
        .expect("program_config account not found in result");

    let resulting_state = ProgramConfig::load(&resulting_program_config.data).unwrap();
    assert_eq!(
        resulting_state.signer_configs[0].signer,
        new_signer.to_bytes().into()
    );
    assert_eq!(resulting_state.seqno_instruction_data_offset.offset, 0);
    assert_eq!(resulting_state.seqno_instruction_data_offset.length, 0);
}

#[test]
fn test_update_program_memcmp_as_delegate_authority() {
    use crate::state::{ProgramConfig, ProgramStatus};

    let program_id = Pubkey::new_unique();
    let mollusk = Mollusk::new(&program_id, "target/deploy/maker_registry");

    let delegate_authority = Pubkey::new_unique();
    let config_key = Pubkey::new_unique();
    let target_program_key = Pubkey::new_unique();
    let executable_data_key = Pubkey::new_unique();
    let program_config_key = Pubkey::new_unique();

    let (program_account, executable_data_account) =
        make_upgrade_authority_accounts(&Pubkey::new_unique(), &executable_data_key);

    let authority_account = Account::new(10_000_000_000, 0, &solana_sdk_ids::system_program::id());
    let config_account = Account {
        lamports: 10_000_000,
        data: vec![0u8; CONFIG_SIZE],
        owner: program_id,
        executable: false,
        rent_epoch: 0,
    };

    let program_config_data = raw_program_config_bytes(
        target_program_key,
        ProgramStatus::Enrolled,
        Some(delegate_authority),
    );

    let program_config_account = Account {
        lamports: 10_000_000,
        data: program_config_data,
        owner: program_id,
        executable: false,
        rent_epoch: 0,
    };

    let instruction_data = update_program_memcmp_instruction_data(12, 4);
    let instruction = Instruction::new_with_bytes(
        program_id,
        &instruction_data,
        vec![
            AccountMeta::new_readonly(delegate_authority, true),
            AccountMeta::new(program_config_key, false),
            AccountMeta::new_readonly(config_key, false),
            AccountMeta::new_readonly(target_program_key, false),
            AccountMeta::new_readonly(executable_data_key, false),
        ],
    );

    let accounts = vec![
        (delegate_authority, authority_account),
        (program_config_key, program_config_account),
        (config_key, config_account),
        (target_program_key, program_account),
        (executable_data_key, executable_data_account),
    ];

    let result =
        mollusk.process_and_validate_instruction(&instruction, &accounts, &[Check::success()]);

    let (_, resulting_program_config) = result
        .resulting_accounts
        .iter()
        .find(|(k, _)| k == &program_config_key)
        .expect("program_config account not found in result");

    let resulting_state = ProgramConfig::load(&resulting_program_config.data).unwrap();
    assert_eq!(resulting_state.seqno_instruction_data_offset.offset, 12);
    assert_eq!(resulting_state.seqno_instruction_data_offset.length, 4);
    assert_eq!(resulting_state.signer_configs[0].signer, [0u8; 32].into());
}

#[test]
fn test_update_program_signer_as_override_authority() {
    use crate::state::{Config, ProgramConfig, ProgramStatus};

    let program_id = Pubkey::new_unique();
    let mollusk = Mollusk::new(&program_id, "target/deploy/maker_registry");

    let override_authority = Pubkey::new_unique();
    let config_key = Pubkey::new_unique();
    let target_program_key = Pubkey::new_unique();
    let executable_data_key = Pubkey::new_unique();
    let program_config_key = Pubkey::new_unique();
    let new_signer = Pubkey::new_unique();

    let mut config_struct = Config::default();
    config_struct.override_authority = override_authority.to_bytes().into();
    let config_data = serialize_bytes(&config_struct);

    let (program_account, executable_data_account) =
        make_upgrade_authority_accounts(&Pubkey::new_unique(), &executable_data_key);

    let authority_account = Account::new(10_000_000_000, 0, &solana_sdk_ids::system_program::id());
    let config_account = Account {
        lamports: 10_000_000,
        data: config_data,
        owner: program_id,
        executable: false,
        rent_epoch: 0,
    };

    let program_config_data =
        raw_program_config_bytes(target_program_key, ProgramStatus::Enrolled, None);

    let program_config_account = Account {
        lamports: 10_000_000,
        data: program_config_data,
        owner: program_id,
        executable: false,
        rent_epoch: 0,
    };

    let instruction_data = update_program_signer_instruction_data(0, new_signer);
    let instruction = Instruction::new_with_bytes(
        program_id,
        &instruction_data,
        vec![
            AccountMeta::new_readonly(override_authority, true),
            AccountMeta::new(program_config_key, false),
            AccountMeta::new_readonly(config_key, false),
            AccountMeta::new_readonly(target_program_key, false),
            AccountMeta::new_readonly(executable_data_key, false),
        ],
    );

    let accounts = vec![
        (override_authority, authority_account),
        (program_config_key, program_config_account),
        (config_key, config_account),
        (target_program_key, program_account),
        (executable_data_key, executable_data_account),
    ];

    let result =
        mollusk.process_and_validate_instruction(&instruction, &accounts, &[Check::success()]);

    let (_, resulting_program_config) = result
        .resulting_accounts
        .iter()
        .find(|(k, _)| k == &program_config_key)
        .expect("program_config account not found in result");

    let resulting_state = ProgramConfig::load(&resulting_program_config.data).unwrap();
    assert_eq!(
        resulting_state.signer_configs[0].signer,
        new_signer.to_bytes().into()
    );
}

#[test]
fn test_update_program_signer_delete_slot() {
    use crate::state::{ProgramConfig, ProgramStatus};

    let program_id = Pubkey::new_unique();
    let mollusk = Mollusk::new(&program_id, "target/deploy/maker_registry");

    let authority = Pubkey::new_unique();
    let config_key = Pubkey::new_unique();
    let target_program_key = Pubkey::new_unique();
    let executable_data_key = Pubkey::new_unique();
    let program_config_key = Pubkey::new_unique();

    let (program_account, executable_data_account) =
        make_upgrade_authority_accounts(&authority, &executable_data_key);

    let mut program_config_struct = ProgramConfig::default();
    program_config_struct.program_id = target_program_key.to_bytes().into();
    program_config_struct.status = ProgramStatus::Enrolled;
    program_config_struct.signer_configs[5].signer = Pubkey::new_unique().to_bytes().into();
    let program_config_data = serialize_bytes(&program_config_struct);

    let program_config_account = Account {
        lamports: 10_000_000,
        data: program_config_data,
        owner: program_id,
        executable: false,
        rent_epoch: 0,
    };
    let authority_account = Account::new(1_000_000, 0, &solana_sdk_ids::system_program::id());
    let config_account = Account {
        lamports: 10_000_000,
        data: vec![0u8; CONFIG_SIZE],
        owner: program_id,
        executable: false,
        rent_epoch: 0,
    };

    let instruction = Instruction::new_with_bytes(
        program_id,
        &update_program_signer_instruction_data(5, Pubkey::new_from_array([0u8; 32])),
        vec![
            AccountMeta::new_readonly(authority, true),
            AccountMeta::new(program_config_key, false),
            AccountMeta::new_readonly(config_key, false),
            AccountMeta::new_readonly(target_program_key, false),
            AccountMeta::new_readonly(executable_data_key, false),
        ],
    );

    let accounts = vec![
        (authority, authority_account),
        (program_config_key, program_config_account),
        (config_key, config_account),
        (target_program_key, program_account),
        (executable_data_key, executable_data_account),
    ];

    let result =
        mollusk.process_and_validate_instruction(&instruction, &accounts, &[Check::success()]);

    let (_, resulting_program_config) = result
        .resulting_accounts
        .iter()
        .find(|(k, _)| k == &program_config_key)
        .expect("program_config account not found in result");

    let resulting_state = ProgramConfig::load(&resulting_program_config.data).unwrap();
    assert_eq!(resulting_state.signer_configs[5].signer, [0u8; 32].into());
}

#[test]
fn test_update_market_config_delete_clears_market_id() {
    use crate::state::{ProgramConfig, ProgramStatus};

    let program_id = Pubkey::new_unique();
    let mollusk = Mollusk::new(&program_id, "target/deploy/maker_registry");

    let authority = Pubkey::new_unique();
    let config_key = Pubkey::new_unique();
    let target_program_key = Pubkey::new_unique();
    let executable_data_key = Pubkey::new_unique();
    let program_config_key = Pubkey::new_unique();
    let index = 7u8;

    let (program_account, executable_data_account) =
        make_upgrade_authority_accounts(&authority, &executable_data_key);

    let mut program_config_struct = ProgramConfig::default();
    program_config_struct.program_id = target_program_key.to_bytes().into();
    program_config_struct.status = ProgramStatus::Enrolled;
    program_config_struct.market_configs[index as usize].market_id =
        derived_market_id(target_program_key, index).into();
    program_config_struct.market_configs[index as usize].writable_account = [9u8; 32].into();
    let program_config_data = serialize_bytes(&program_config_struct);

    let program_config_account = Account {
        lamports: 10_000_000,
        data: program_config_data,
        owner: program_id,
        executable: false,
        rent_epoch: 0,
    };
    let authority_account = Account::new(1_000_000, 0, &solana_sdk_ids::system_program::id());
    let config_account = Account {
        lamports: 10_000_000,
        data: vec![0u8; CONFIG_SIZE],
        owner: program_id,
        executable: false,
        rent_epoch: 0,
    };

    let instruction = Instruction::new_with_bytes(
        program_id,
        &update_market_instruction_data(index, [0u8; 32]),
        vec![
            AccountMeta::new_readonly(authority, true),
            AccountMeta::new(program_config_key, false),
            AccountMeta::new_readonly(config_key, false),
            AccountMeta::new_readonly(target_program_key, false),
            AccountMeta::new_readonly(executable_data_key, false),
        ],
    );

    let accounts = vec![
        (authority, authority_account),
        (program_config_key, program_config_account),
        (config_key, config_account),
        (target_program_key, program_account),
        (executable_data_key, executable_data_account),
    ];

    let result =
        mollusk.process_and_validate_instruction(&instruction, &accounts, &[Check::success()]);

    let (_, resulting_program_config) = result
        .resulting_accounts
        .iter()
        .find(|(k, _)| k == &program_config_key)
        .expect("program_config account not found in result");

    let resulting_state = ProgramConfig::load(&resulting_program_config.data).unwrap();
    assert_eq!(
        resulting_state.market_configs[index as usize].writable_account,
        [0u8; 32].into()
    );
    assert_eq!(
        resulting_state.market_configs[index as usize].market_id,
        [0u8; 32].into()
    );
}

#[test]
fn test_update_program_memcmp_rejects_invalid_instruction_data() {
    use crate::state::ProgramStatus;

    let program_id = Pubkey::new_unique();
    let mollusk = Mollusk::new(&program_id, "target/deploy/maker_registry");

    let authority = Pubkey::new_unique();
    let config_key = Pubkey::new_unique();
    let target_program_key = Pubkey::new_unique();
    let executable_data_key = Pubkey::new_unique();
    let program_config_key = Pubkey::new_unique();

    let (program_account, executable_data_account) =
        make_upgrade_authority_accounts(&authority, &executable_data_key);

    let program_config_data =
        raw_program_config_bytes(target_program_key, ProgramStatus::Enrolled, None);

    let program_config_account = Account {
        lamports: 10_000_000,
        data: program_config_data,
        owner: program_id,
        executable: false,
        rent_epoch: 0,
    };
    let authority_account = Account::new(1_000_000, 0, &solana_sdk_ids::system_program::id());
    let config_account = Account {
        lamports: 10_000_000,
        data: vec![0u8; CONFIG_SIZE],
        owner: program_id,
        executable: false,
        rent_epoch: 0,
    };

    let instruction = Instruction::new_with_bytes(
        program_id,
        &[9u8, 1u8, 2u8],
        vec![
            AccountMeta::new_readonly(authority, true),
            AccountMeta::new(program_config_key, false),
            AccountMeta::new_readonly(config_key, false),
            AccountMeta::new_readonly(target_program_key, false),
            AccountMeta::new_readonly(executable_data_key, false),
        ],
    );

    let accounts = vec![
        (authority, authority_account),
        (program_config_key, program_config_account),
        (config_key, config_account),
        (target_program_key, program_account),
        (executable_data_key, executable_data_account),
    ];

    let result = mollusk.process_instruction(&instruction, &accounts);
    assert_program_error(result, InstructionError::InvalidInstructionData);
}

#[test]
fn test_update_program_signer_rejects_out_of_range_index() {
    use crate::state::ProgramStatus;

    let program_id = Pubkey::new_unique();
    let mollusk = Mollusk::new(&program_id, "target/deploy/maker_registry");

    let authority = Pubkey::new_unique();
    let config_key = Pubkey::new_unique();
    let target_program_key = Pubkey::new_unique();
    let executable_data_key = Pubkey::new_unique();
    let program_config_key = Pubkey::new_unique();

    let (program_account, executable_data_account) =
        make_upgrade_authority_accounts(&authority, &executable_data_key);

    let program_config_data =
        raw_program_config_bytes(target_program_key, ProgramStatus::Enrolled, None);

    let program_config_account = Account {
        lamports: 10_000_000,
        data: program_config_data,
        owner: program_id,
        executable: false,
        rent_epoch: 0,
    };
    let authority_account = Account::new(1_000_000, 0, &solana_sdk_ids::system_program::id());
    let config_account = Account {
        lamports: 10_000_000,
        data: vec![0u8; CONFIG_SIZE],
        owner: program_id,
        executable: false,
        rent_epoch: 0,
    };

    let instruction = Instruction::new_with_bytes(
        program_id,
        &update_program_signer_instruction_data(32, Pubkey::new_unique()),
        vec![
            AccountMeta::new_readonly(authority, true),
            AccountMeta::new(program_config_key, false),
            AccountMeta::new_readonly(config_key, false),
            AccountMeta::new_readonly(target_program_key, false),
            AccountMeta::new_readonly(executable_data_key, false),
        ],
    );

    let accounts = vec![
        (authority, authority_account),
        (program_config_key, program_config_account),
        (config_key, config_account),
        (target_program_key, program_account),
        (executable_data_key, executable_data_account),
    ];

    let result = mollusk.process_instruction(&instruction, &accounts);
    assert_program_error(result, InstructionError::InvalidInstructionData);
}

#[test]
fn test_admin_change_authority_rejects_invalid_field() {
    use crate::state::Config;

    let program_id = Pubkey::new_unique();
    let mollusk = Mollusk::new(&program_id, "target/deploy/maker_registry");

    let admin = Pubkey::new_unique();
    let config_key = Pubkey::new_unique();
    let new_authority = Pubkey::new_unique();

    let mut config_struct = Config::default();
    config_struct.admin_authority = admin.to_bytes().into();
    let config_data = serialize_bytes(&config_struct);

    let admin_account = Account::new(1_000_000, 0, &solana_sdk_ids::system_program::id());
    let config_account = Account {
        lamports: 10_000_000,
        data: config_data,
        owner: program_id,
        executable: false,
        rent_epoch: 0,
    };

    let instruction = Instruction::new_with_bytes(
        program_id,
        &[&[6u8, 3u8][..], new_authority.as_ref()].concat(),
        vec![
            AccountMeta::new_readonly(admin, true),
            AccountMeta::new(config_key, false),
        ],
    );

    let accounts = vec![(admin, admin_account), (config_key, config_account)];

    let result = mollusk.process_instruction(&instruction, &accounts);
    assert_program_error(result, InstructionError::InvalidInstructionData);
}

#[test]
fn test_upgrade_authority_unenroll_rejects_unauthorized_authority() {
    use crate::state::{ProgramConfig, ProgramStatus};
    let program_id = Pubkey::new_unique();
    let mollusk = Mollusk::new(&program_id, "target/deploy/maker_registry");

    let unauthorized_authority = Pubkey::new_unique();
    let upgrade_authority = Pubkey::new_unique();
    let config_key = Pubkey::new_unique();
    let target_program_key = Pubkey::new_unique();
    let executable_data_key = Pubkey::new_unique();
    let program_config_key = Pubkey::new_unique();

    let (program_account, executable_data_account) =
        make_upgrade_authority_accounts(&upgrade_authority, &executable_data_key);

    let mut program_config_struct = ProgramConfig::default();
    program_config_struct.program_id = target_program_key.to_bytes().into();
    program_config_struct.status = ProgramStatus::Active;
    let program_config_data = serialize_bytes(&program_config_struct);

    let program_config_account = Account {
        lamports: 10_000_000,
        data: program_config_data,
        owner: program_id,
        executable: false,
        rent_epoch: 0,
    };
    let authority_account = Account::new(10_000_000_000, 0, &solana_sdk_ids::system_program::id());
    let config_account = Account {
        lamports: 10_000_000,
        data: vec![0u8; CONFIG_SIZE],
        owner: program_id,
        executable: false,
        rent_epoch: 0,
    };

    let instruction = Instruction::new_with_bytes(
        program_id,
        &[3u8],
        vec![
            AccountMeta::new_readonly(unauthorized_authority, true),
            AccountMeta::new_readonly(config_key, false),
            AccountMeta::new_readonly(target_program_key, false),
            AccountMeta::new_readonly(executable_data_key, false),
            AccountMeta::new(program_config_key, false),
        ],
    );

    let accounts = vec![
        (unauthorized_authority, authority_account),
        (config_key, config_account),
        (target_program_key, program_account),
        (executable_data_key, executable_data_account),
        (program_config_key, program_config_account),
    ];

    let result = mollusk.process_instruction(&instruction, &accounts);
    assert_program_error(result, InstructionError::MissingRequiredSignature);
}

#[test]
fn test_assign_delegate_authority_rejects_unauthorized_authority() {
    use crate::state::{Config, ProgramConfig};
    let program_id = Pubkey::new_unique();
    let mollusk = Mollusk::new(&program_id, "target/deploy/maker_registry");

    let unauthorized_authority = Pubkey::new_unique();
    let delegate_authority = Pubkey::new_unique();
    let target_program_key = Pubkey::new_unique();
    let executable_data_key = Pubkey::new_unique();
    let config_key = Pubkey::new_unique();
    let program_config_key = Pubkey::new_unique();

    let mut config_struct = Config::default();
    config_struct.override_authority = Pubkey::new_unique().to_bytes().into();
    let config_data = serialize_bytes(&config_struct);

    let mut program_config_struct = ProgramConfig::default();
    program_config_struct.program_id = target_program_key.to_bytes().into();
    let program_config_data = serialize_bytes(&program_config_struct);

    let config_account = Account {
        lamports: 10_000_000,
        data: config_data,
        owner: program_id,
        executable: false,
        rent_epoch: 0,
    };
    let program_config_account = Account {
        lamports: 10_000_000,
        data: program_config_data,
        owner: program_id,
        executable: false,
        rent_epoch: 0,
    };
    let authority_account = Account::new(1_000_000, 0, &solana_sdk_ids::system_program::id());
    let (program_account, executable_data_account) =
        make_upgrade_authority_accounts(&Pubkey::new_unique(), &executable_data_key);

    let instruction = Instruction::new_with_bytes(
        program_id,
        &[&[7u8], delegate_authority.as_ref()].concat(),
        vec![
            AccountMeta::new_readonly(unauthorized_authority, true),
            AccountMeta::new_readonly(config_key, false),
            AccountMeta::new(program_config_key, false),
            AccountMeta::new_readonly(target_program_key, false),
            AccountMeta::new_readonly(executable_data_key, false),
        ],
    );

    let accounts = vec![
        (unauthorized_authority, authority_account),
        (config_key, config_account),
        (program_config_key, program_config_account),
        (target_program_key, program_account),
        (executable_data_key, executable_data_account),
    ];

    let result = mollusk.process_instruction(&instruction, &accounts);
    assert_program_error(result, InstructionError::MissingRequiredSignature);
}

#[test]
fn test_activate_rejects_non_status_authority() {
    use crate::state::{Config, ProgramConfig, ProgramStatus};
    let program_id = Pubkey::new_unique();
    let mollusk = Mollusk::new(&program_id, "target/deploy/maker_registry");

    let authority = Pubkey::new_unique();
    let config_key = Pubkey::new_unique();
    let program_config_key = Pubkey::new_unique();

    let mut config_struct = Config::default();
    config_struct.status_authority = Pubkey::new_unique().to_bytes().into();
    let config_data = serialize_bytes(&config_struct);

    let mut program_config_struct = ProgramConfig::default();
    program_config_struct.status = ProgramStatus::Enrolled;
    let program_config_data = serialize_bytes(&program_config_struct);

    let config_account = Account {
        lamports: 10_000_000,
        data: config_data,
        owner: program_id,
        executable: false,
        rent_epoch: 0,
    };
    let program_config_account = Account {
        lamports: 10_000_000,
        data: program_config_data,
        owner: program_id,
        executable: false,
        rent_epoch: 0,
    };
    let authority_account = Account::new(1_000_000, 0, &solana_sdk_ids::system_program::id());

    let instruction = Instruction::new_with_bytes(
        program_id,
        &[4u8],
        vec![
            AccountMeta::new_readonly(authority, true),
            AccountMeta::new_readonly(config_key, false),
            AccountMeta::new(program_config_key, false),
        ],
    );

    let accounts = vec![
        (authority, authority_account),
        (config_key, config_account),
        (program_config_key, program_config_account),
    ];

    let result = mollusk.process_instruction(&instruction, &accounts);
    assert_program_error(result, InstructionError::MissingRequiredSignature);
}

#[test]
fn test_admin_change_authority_rejects_non_admin() {
    use crate::state::Config;
    let program_id = Pubkey::new_unique();
    let mollusk = Mollusk::new(&program_id, "target/deploy/maker_registry");

    let non_admin = Pubkey::new_unique();
    let config_key = Pubkey::new_unique();
    let new_authority = Pubkey::new_unique();

    let mut config_struct = Config::default();
    config_struct.admin_authority = Pubkey::new_unique().to_bytes().into();
    let config_data = serialize_bytes(&config_struct);

    let config_account = Account {
        lamports: 10_000_000,
        data: config_data,
        owner: program_id,
        executable: false,
        rent_epoch: 0,
    };
    let authority_account = Account::new(1_000_000, 0, &solana_sdk_ids::system_program::id());

    let instruction = Instruction::new_with_bytes(
        program_id,
        &[&[6u8, 1u8], new_authority.as_ref()].concat(),
        vec![
            AccountMeta::new_readonly(non_admin, true),
            AccountMeta::new(config_key, false),
        ],
    );

    let accounts = vec![(non_admin, authority_account), (config_key, config_account)];

    let result = mollusk.process_instruction(&instruction, &accounts);
    assert_program_error(result, InstructionError::MissingRequiredSignature);
}
