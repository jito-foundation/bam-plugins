use bincode::serialize;
use maker_registry::state::{
    Config, MarketUpdateMode, ProgramConfig, ProgramStatus, CONFIG_SIZE, PROGRAM_CONFIG_SIZE,
};
use solana_loader_v3_interface::state::UpgradeableLoaderState;
use solana_program_test::{ProgramTest, ProgramTestContext};
use solana_sdk::{
    account::Account,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_program, sysvar,
    transaction::Transaction,
};
use std::path::PathBuf;

struct InitializedProgramContext {
    context: ProgramTestContext,
    program_id: Pubkey,
    config_pda: Pubkey,
}

struct EnrolledProgramContext {
    program_config_pda: Pubkey,
    target_program_id: Pubkey,
    executable_data_key: Pubkey,
}

fn configure_program_test(program_id: Pubkey) -> ProgramTest {
    let bpf_out_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/deploy");
    std::env::set_var("BPF_OUT_DIR", &bpf_out_dir);

    let mut program_test = ProgramTest::default();
    program_test.add_program("maker_registry", program_id, None);
    program_test
}

async fn setup_initialized_program() -> InitializedProgramContext {
    let program_id = Pubkey::new_unique();
    let context = configure_program_test(program_id)
        .start_with_context()
        .await;

    let (config_pda, bump) = Pubkey::find_program_address(&[b"config"], &program_id);

    let instruction = Instruction::new_with_bytes(
        program_id,
        &[0, bump],
        vec![
            AccountMeta::new(context.payer.pubkey(), true),
            AccountMeta::new(config_pda, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ],
    );

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&context.payer.pubkey()),
        &[&context.payer],
        context.last_blockhash,
    );

    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    InitializedProgramContext {
        context,
        program_id,
        config_pda,
    }
}

fn make_upgrade_authority_accounts(
    authority: &Pubkey,
    executable_data_key: &Pubkey,
) -> (Account, Account) {
    let program_account_data = serialize(&UpgradeableLoaderState::Program {
        programdata_address: *executable_data_key,
    })
    .unwrap();

    let exec_data_bytes = serialize(&UpgradeableLoaderState::ProgramData {
        slot: 0,
        upgrade_authority_address: Some(*authority),
    })
    .unwrap();

    let program_account = Account {
        lamports: 1_000_000,
        data: program_account_data,
        owner: solana_sdk_ids::bpf_loader_upgradeable::id(),
        executable: false,
        rent_epoch: 0,
    };
    let executable_data_account = Account {
        lamports: 1_000_000,
        data: exec_data_bytes,
        owner: solana_sdk_ids::bpf_loader_upgradeable::id(),
        executable: false,
        rent_epoch: 0,
    };

    (program_account, executable_data_account)
}

async fn enroll_program(
    context: &mut ProgramTestContext,
    program_id: Pubkey,
    config_pda: Pubkey,
    authority: Pubkey,
    extra_signer: Option<&Keypair>,
    market_update_mode: MarketUpdateMode,
) -> EnrolledProgramContext {
    let target_program_id = Pubkey::new_unique();
    let executable_data_key = Pubkey::new_unique();
    let (program_account, executable_data_account) =
        make_upgrade_authority_accounts(&authority, &executable_data_key);

    context.set_account(&target_program_id, &program_account.into());
    context.set_account(&executable_data_key, &executable_data_account.into());

    let (program_config_pda, bump) =
        Pubkey::find_program_address(&[&target_program_id.to_bytes()], &program_id);

    let mut instruction_data = vec![1u8];
    instruction_data.extend_from_slice(&target_program_id.to_bytes());
    instruction_data.push(bump);
    instruction_data.push(market_update_mode.as_u8());

    let instruction = Instruction::new_with_bytes(
        program_id,
        &instruction_data,
        vec![
            AccountMeta::new(authority, true),
            AccountMeta::new(program_config_pda, false),
            AccountMeta::new_readonly(config_pda, false),
            AccountMeta::new_readonly(target_program_id, false),
            AccountMeta::new_readonly(executable_data_key, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ],
    );

    let mut signers: Vec<&Keypair> = vec![&context.payer];
    if let Some(extra_signer) = extra_signer {
        signers.push(extra_signer);
    }

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&context.payer.pubkey()),
        &signers,
        context.last_blockhash,
    );

    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    EnrolledProgramContext {
        program_config_pda,
        target_program_id,
        executable_data_key,
    }
}

fn derived_market_id(target_program_id: Pubkey, index: u8) -> [u8; 32] {
    let mut market_id = [0u8; 32];
    market_id[..31].copy_from_slice(&target_program_id.to_bytes()[..31]);
    market_id[31] = index;
    market_id
}

async fn fetch_program_config(
    context: &mut ProgramTestContext,
    program_config_pda: Pubkey,
) -> Vec<u8> {
    let program_config_account = context
        .banks_client
        .get_account(program_config_pda)
        .await
        .unwrap()
        .expect("program_config PDA should exist");
    program_config_account.data
}

async fn store_override_authority(
    context: &mut ProgramTestContext,
    config_pda: Pubkey,
    override_authority: Pubkey,
) {
    let config_account = context
        .banks_client
        .get_account(config_pda)
        .await
        .unwrap()
        .expect("config PDA should exist");
    let mut updated_account = config_account;
    let config = Config::load_mut(&mut updated_account.data).unwrap();
    config.override_authority = override_authority.to_bytes().into();

    context.set_account(&config_pda, &updated_account.into());
}

async fn assign_delegate_authority(
    context: &mut ProgramTestContext,
    program_id: Pubkey,
    config_pda: Pubkey,
    enrolled: &EnrolledProgramContext,
    signer_pubkey: Pubkey,
    extra_signer: Option<&Keypair>,
    delegate_authority: Pubkey,
) {
    let mut assign_delegate_data = vec![7u8];
    assign_delegate_data.extend_from_slice(&delegate_authority.to_bytes());

    let assign_delegate_instruction = Instruction::new_with_bytes(
        program_id,
        &assign_delegate_data,
        vec![
            AccountMeta::new_readonly(signer_pubkey, true),
            AccountMeta::new_readonly(config_pda, false),
            AccountMeta::new(enrolled.program_config_pda, false),
            AccountMeta::new_readonly(enrolled.target_program_id, false),
            AccountMeta::new_readonly(enrolled.executable_data_key, false),
        ],
    );

    let mut signers: Vec<&Keypair> = vec![&context.payer];
    if let Some(extra_signer) = extra_signer {
        signers.push(extra_signer);
    }

    let transaction = Transaction::new_signed_with_payer(
        &[assign_delegate_instruction],
        Some(&context.payer.pubkey()),
        &signers,
        context.last_blockhash,
    );

    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();
}

async fn update_program_signer(
    context: &mut ProgramTestContext,
    program_id: Pubkey,
    config_pda: Pubkey,
    enrolled: &EnrolledProgramContext,
    signer_pubkey: Pubkey,
    extra_signer: Option<&Keypair>,
    index: u8,
    new_signer: Pubkey,
) {
    let mut instruction_data = vec![8u8, index];
    instruction_data.extend_from_slice(&new_signer.to_bytes());

    let instruction = Instruction::new_with_bytes(
        program_id,
        &instruction_data,
        vec![
            AccountMeta::new_readonly(signer_pubkey, true),
            AccountMeta::new(enrolled.program_config_pda, false),
            AccountMeta::new_readonly(config_pda, false),
            AccountMeta::new_readonly(enrolled.target_program_id, false),
            AccountMeta::new_readonly(enrolled.executable_data_key, false),
        ],
    );

    let mut signers: Vec<&Keypair> = vec![&context.payer];
    if let Some(extra_signer) = extra_signer {
        signers.push(extra_signer);
    }

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&context.payer.pubkey()),
        &signers,
        context.last_blockhash,
    );

    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();
}

async fn update_program_memcmp(
    context: &mut ProgramTestContext,
    program_id: Pubkey,
    config_pda: Pubkey,
    enrolled: &EnrolledProgramContext,
    signer_pubkey: Pubkey,
    extra_signer: Option<&Keypair>,
    offset: u16,
    length: u16,
) {
    let mut instruction_data = vec![9u8];
    instruction_data.extend_from_slice(&offset.to_le_bytes());
    instruction_data.extend_from_slice(&length.to_le_bytes());

    let instruction = Instruction::new_with_bytes(
        program_id,
        &instruction_data,
        vec![
            AccountMeta::new_readonly(signer_pubkey, true),
            AccountMeta::new(enrolled.program_config_pda, false),
            AccountMeta::new_readonly(config_pda, false),
            AccountMeta::new_readonly(enrolled.target_program_id, false),
            AccountMeta::new_readonly(enrolled.executable_data_key, false),
        ],
    );

    let mut signers: Vec<&Keypair> = vec![&context.payer];
    if let Some(extra_signer) = extra_signer {
        signers.push(extra_signer);
    }

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&context.payer.pubkey()),
        &signers,
        context.last_blockhash,
    );

    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();
}

#[tokio::test]
async fn init_config_creates_the_config_pda() {
    let InitializedProgramContext {
        context,
        program_id,
        config_pda,
    } = setup_initialized_program().await;
    let context = context;

    let config_account = context
        .banks_client
        .get_account(config_pda)
        .await
        .unwrap()
        .expect("config PDA should exist after init");

    assert_eq!(config_account.owner, program_id);
    assert_eq!(config_account.data.len(), CONFIG_SIZE);
    assert!(config_account.lamports > 0);
}

#[tokio::test]
async fn init_program_config_enrolls_a_program() {
    let InitializedProgramContext {
        mut context,
        program_id,
        config_pda,
    } = setup_initialized_program().await;
    let payer_pubkey = context.payer.pubkey();

    let enrolled = enroll_program(
        &mut context,
        program_id,
        config_pda,
        payer_pubkey,
        None,
        MarketUpdateMode::MultiMarket,
    )
    .await;

    let program_config_account = context
        .banks_client
        .get_account(enrolled.program_config_pda)
        .await
        .unwrap()
        .expect("program_config PDA should exist after enroll");
    let program_config = ProgramConfig::load(&program_config_account.data).unwrap();

    assert_eq!(program_config_account.owner, program_id);
    assert_eq!(program_config_account.data.len(), PROGRAM_CONFIG_SIZE);
    assert_eq!(
        program_config.program_id,
        enrolled.target_program_id.to_bytes().into()
    );
    assert_eq!(program_config.status, ProgramStatus::Enrolled);
    assert_eq!(
        program_config.market_update_mode,
        MarketUpdateMode::MultiMarket
    );
}

#[tokio::test]
async fn init_enroll_and_update_market_config() {
    let InitializedProgramContext {
        mut context,
        program_id,
        config_pda,
    } = setup_initialized_program().await;
    let payer_pubkey = context.payer.pubkey();

    let enrolled = enroll_program(
        &mut context,
        program_id,
        config_pda,
        payer_pubkey,
        None,
        MarketUpdateMode::SingleMarket,
    )
    .await;
    let index = 0u8;
    let mut instruction_data = vec![2u8, index];
    instruction_data.extend_from_slice(&[9u8; 32]);

    let instruction = Instruction::new_with_bytes(
        program_id,
        &instruction_data,
        vec![
            AccountMeta::new_readonly(payer_pubkey, true),
            AccountMeta::new(enrolled.program_config_pda, false),
            AccountMeta::new_readonly(config_pda, false),
            AccountMeta::new_readonly(enrolled.target_program_id, false),
            AccountMeta::new_readonly(enrolled.executable_data_key, false),
        ],
    );

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer_pubkey),
        &[&context.payer],
        context.last_blockhash,
    );

    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    let program_config_account = context
        .banks_client
        .get_account(enrolled.program_config_pda)
        .await
        .unwrap()
        .expect("program_config PDA should exist after update");
    let program_config = ProgramConfig::load(&program_config_account.data).unwrap();

    assert_eq!(
        program_config.market_configs[0].market_id,
        derived_market_id(enrolled.target_program_id, index).into()
    );
    assert_eq!(
        program_config.market_configs[0].writable_account,
        [9u8; 32].into()
    );
    assert_eq!(program_config.seqno_instruction_data_offset.offset, 0);
    assert_eq!(program_config.seqno_instruction_data_offset.length, 0);
    assert_eq!(program_config.signer_configs[0].signer, [0u8; 32].into());
}

#[tokio::test]
async fn init_enroll_assign_delegate_and_update_market_config_as_delegate() {
    let InitializedProgramContext {
        mut context,
        program_id,
        config_pda,
    } = setup_initialized_program().await;
    let payer_pubkey = context.payer.pubkey();

    let enrolled = enroll_program(
        &mut context,
        program_id,
        config_pda,
        payer_pubkey,
        None,
        MarketUpdateMode::SingleMarket,
    )
    .await;
    let delegate_authority = Keypair::new();
    let index = 0u8;
    assign_delegate_authority(
        &mut context,
        program_id,
        config_pda,
        &enrolled,
        payer_pubkey,
        None,
        delegate_authority.pubkey(),
    )
    .await;

    let delegate_account = Account {
        lamports: 1_000_000,
        data: vec![],
        owner: system_program::id(),
        executable: false,
        rent_epoch: 0,
    };
    context.set_account(&delegate_authority.pubkey(), &delegate_account.into());

    let mut update_instruction_data = vec![2u8, index];
    update_instruction_data.extend_from_slice(&[9u8; 32]);

    let update_instruction = Instruction::new_with_bytes(
        program_id,
        &update_instruction_data,
        vec![
            AccountMeta::new_readonly(delegate_authority.pubkey(), true),
            AccountMeta::new(enrolled.program_config_pda, false),
            AccountMeta::new_readonly(config_pda, false),
            AccountMeta::new_readonly(enrolled.target_program_id, false),
            AccountMeta::new_readonly(enrolled.executable_data_key, false),
        ],
    );

    let update_transaction = Transaction::new_signed_with_payer(
        &[update_instruction],
        Some(&payer_pubkey),
        &[&context.payer, &delegate_authority],
        context.last_blockhash,
    );

    context
        .banks_client
        .process_transaction(update_transaction)
        .await
        .unwrap();

    let program_config_account = context
        .banks_client
        .get_account(enrolled.program_config_pda)
        .await
        .unwrap()
        .expect("program_config PDA should exist after delegate update");
    let program_config = ProgramConfig::load(&program_config_account.data).unwrap();

    assert_eq!(
        program_config.delegate_authority,
        delegate_authority.pubkey().to_bytes().into()
    );
    assert_eq!(
        program_config.market_configs[0].market_id,
        derived_market_id(enrolled.target_program_id, index).into()
    );
    assert_eq!(
        program_config.market_configs[0].writable_account,
        [9u8; 32].into()
    );
    assert_eq!(program_config.signer_configs[0].signer, [0u8; 32].into());
}

#[tokio::test]
async fn init_enroll_and_unenroll_as_upgrade_authority() {
    let InitializedProgramContext {
        mut context,
        program_id,
        config_pda,
    } = setup_initialized_program().await;
    let payer_pubkey = context.payer.pubkey();

    let enrolled = enroll_program(
        &mut context,
        program_id,
        config_pda,
        payer_pubkey,
        None,
        MarketUpdateMode::SingleMarket,
    )
    .await;

    let instruction = Instruction::new_with_bytes(
        program_id,
        &[3u8],
        vec![
            AccountMeta::new_readonly(payer_pubkey, true),
            AccountMeta::new_readonly(config_pda, false),
            AccountMeta::new_readonly(enrolled.target_program_id, false),
            AccountMeta::new_readonly(enrolled.executable_data_key, false),
            AccountMeta::new(enrolled.program_config_pda, false),
        ],
    );

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer_pubkey),
        &[&context.payer],
        context.last_blockhash,
    );

    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    let program_config_data = fetch_program_config(&mut context, enrolled.program_config_pda).await;
    let program_config = ProgramConfig::load(&program_config_data).unwrap();
    assert_eq!(program_config.status, ProgramStatus::Unenroll);
}

#[tokio::test]
async fn init_enroll_assign_delegate_and_unenroll_as_delegate() {
    let InitializedProgramContext {
        mut context,
        program_id,
        config_pda,
    } = setup_initialized_program().await;
    let payer_pubkey = context.payer.pubkey();

    let enrolled = enroll_program(
        &mut context,
        program_id,
        config_pda,
        payer_pubkey,
        None,
        MarketUpdateMode::SingleMarket,
    )
    .await;
    let delegate_authority = Keypair::new();
    assign_delegate_authority(
        &mut context,
        program_id,
        config_pda,
        &enrolled,
        payer_pubkey,
        None,
        delegate_authority.pubkey(),
    )
    .await;

    let delegate_account = Account {
        lamports: 1_000_000,
        data: vec![],
        owner: system_program::id(),
        executable: false,
        rent_epoch: 0,
    };
    context.set_account(&delegate_authority.pubkey(), &delegate_account.into());

    let unenroll_instruction = Instruction::new_with_bytes(
        program_id,
        &[3u8],
        vec![
            AccountMeta::new_readonly(delegate_authority.pubkey(), true),
            AccountMeta::new_readonly(config_pda, false),
            AccountMeta::new_readonly(enrolled.target_program_id, false),
            AccountMeta::new_readonly(enrolled.executable_data_key, false),
            AccountMeta::new(enrolled.program_config_pda, false),
        ],
    );

    let unenroll_transaction = Transaction::new_signed_with_payer(
        &[unenroll_instruction],
        Some(&payer_pubkey),
        &[&context.payer, &delegate_authority],
        context.last_blockhash,
    );

    context
        .banks_client
        .process_transaction(unenroll_transaction)
        .await
        .unwrap();

    let program_config_data = fetch_program_config(&mut context, enrolled.program_config_pda).await;
    let program_config = ProgramConfig::load(&program_config_data).unwrap();
    assert_eq!(
        program_config.delegate_authority,
        delegate_authority.pubkey().to_bytes().into()
    );
    assert_eq!(program_config.status, ProgramStatus::Unenroll);
}

#[tokio::test]
async fn init_enroll_and_update_market_config_as_override_authority() {
    let InitializedProgramContext {
        mut context,
        program_id,
        config_pda,
    } = setup_initialized_program().await;
    let override_authority = Keypair::new();

    let override_account = Account {
        lamports: 10_000_000_000,
        data: vec![],
        owner: system_program::id(),
        executable: false,
        rent_epoch: 0,
    };
    context.set_account(&override_authority.pubkey(), &override_account.into());
    store_override_authority(&mut context, config_pda, override_authority.pubkey()).await;

    let enrolled = enroll_program(
        &mut context,
        program_id,
        config_pda,
        override_authority.pubkey(),
        Some(&override_authority),
        MarketUpdateMode::SingleMarket,
    )
    .await;
    let index = 0u8;
    let mut instruction_data = vec![2u8, index];
    instruction_data.extend_from_slice(&[9u8; 32]);

    let instruction = Instruction::new_with_bytes(
        program_id,
        &instruction_data,
        vec![
            AccountMeta::new_readonly(override_authority.pubkey(), true),
            AccountMeta::new(enrolled.program_config_pda, false),
            AccountMeta::new_readonly(config_pda, false),
            AccountMeta::new_readonly(enrolled.target_program_id, false),
            AccountMeta::new_readonly(enrolled.executable_data_key, false),
        ],
    );

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&context.payer.pubkey()),
        &[&context.payer, &override_authority],
        context.last_blockhash,
    );

    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    let program_config_data = fetch_program_config(&mut context, enrolled.program_config_pda).await;
    let program_config = ProgramConfig::load(&program_config_data).unwrap();
    assert_eq!(program_config.status, ProgramStatus::Enrolled);
    assert_eq!(
        program_config.market_configs[0].market_id,
        derived_market_id(enrolled.target_program_id, index).into()
    );
    assert_eq!(
        program_config.market_configs[0].writable_account,
        [9u8; 32].into()
    );
    assert_eq!(program_config.signer_configs[0].signer, [0u8; 32].into());
}

#[tokio::test]
async fn init_enroll_and_update_program_signer() {
    let InitializedProgramContext {
        mut context,
        program_id,
        config_pda,
    } = setup_initialized_program().await;
    let payer_pubkey = context.payer.pubkey();
    let new_signer = Pubkey::new_unique();

    let enrolled = enroll_program(
        &mut context,
        program_id,
        config_pda,
        payer_pubkey,
        None,
        MarketUpdateMode::SingleMarket,
    )
    .await;
    update_program_signer(
        &mut context,
        program_id,
        config_pda,
        &enrolled,
        payer_pubkey,
        None,
        0,
        new_signer,
    )
    .await;

    let program_config_data = fetch_program_config(&mut context, enrolled.program_config_pda).await;
    let program_config = ProgramConfig::load(&program_config_data).unwrap();

    assert_eq!(
        program_config.signer_configs[0].signer,
        new_signer.to_bytes().into()
    );
    assert_eq!(program_config.seqno_instruction_data_offset.offset, 0);
    assert_eq!(program_config.seqno_instruction_data_offset.length, 0);
}

#[tokio::test]
async fn init_enroll_assign_delegate_and_update_program_memcmp_as_delegate() {
    let InitializedProgramContext {
        mut context,
        program_id,
        config_pda,
    } = setup_initialized_program().await;
    let payer_pubkey = context.payer.pubkey();

    let enrolled = enroll_program(
        &mut context,
        program_id,
        config_pda,
        payer_pubkey,
        None,
        MarketUpdateMode::SingleMarket,
    )
    .await;
    let delegate_authority = Keypair::new();
    assign_delegate_authority(
        &mut context,
        program_id,
        config_pda,
        &enrolled,
        payer_pubkey,
        None,
        delegate_authority.pubkey(),
    )
    .await;

    let delegate_account = Account {
        lamports: 1_000_000,
        data: vec![],
        owner: system_program::id(),
        executable: false,
        rent_epoch: 0,
    };
    context.set_account(&delegate_authority.pubkey(), &delegate_account.into());

    update_program_memcmp(
        &mut context,
        program_id,
        config_pda,
        &enrolled,
        delegate_authority.pubkey(),
        Some(&delegate_authority),
        14,
        2,
    )
    .await;

    let program_config_data = fetch_program_config(&mut context, enrolled.program_config_pda).await;
    let program_config = ProgramConfig::load(&program_config_data).unwrap();

    assert_eq!(
        program_config.delegate_authority,
        delegate_authority.pubkey().to_bytes().into()
    );
    assert_eq!(program_config.seqno_instruction_data_offset.offset, 14);
    assert_eq!(program_config.seqno_instruction_data_offset.length, 2);
    assert_eq!(program_config.signer_configs[0].signer, [0u8; 32].into());
}

#[tokio::test]
async fn init_enroll_and_update_program_signer_as_override_authority() {
    let InitializedProgramContext {
        mut context,
        program_id,
        config_pda,
    } = setup_initialized_program().await;
    let override_authority = Keypair::new();
    let new_signer = Pubkey::new_unique();

    let override_account = Account {
        lamports: 10_000_000_000,
        data: vec![],
        owner: system_program::id(),
        executable: false,
        rent_epoch: 0,
    };
    context.set_account(&override_authority.pubkey(), &override_account.into());
    store_override_authority(&mut context, config_pda, override_authority.pubkey()).await;

    let enrolled = enroll_program(
        &mut context,
        program_id,
        config_pda,
        override_authority.pubkey(),
        Some(&override_authority),
        MarketUpdateMode::SingleMarket,
    )
    .await;

    update_program_signer(
        &mut context,
        program_id,
        config_pda,
        &enrolled,
        override_authority.pubkey(),
        Some(&override_authority),
        0,
        new_signer,
    )
    .await;

    let program_config_data = fetch_program_config(&mut context, enrolled.program_config_pda).await;
    let program_config = ProgramConfig::load(&program_config_data).unwrap();

    assert_eq!(program_config.status, ProgramStatus::Enrolled);
    assert_eq!(
        program_config.signer_configs[0].signer,
        new_signer.to_bytes().into()
    );
}

#[tokio::test]
async fn init_enroll_and_delete_program_signer() {
    let InitializedProgramContext {
        mut context,
        program_id,
        config_pda,
    } = setup_initialized_program().await;
    let payer_pubkey = context.payer.pubkey();

    let enrolled = enroll_program(
        &mut context,
        program_id,
        config_pda,
        payer_pubkey,
        None,
        MarketUpdateMode::SingleMarket,
    )
    .await;
    update_program_signer(
        &mut context,
        program_id,
        config_pda,
        &enrolled,
        payer_pubkey,
        None,
        3,
        Pubkey::new_unique(),
    )
    .await;
    update_program_signer(
        &mut context,
        program_id,
        config_pda,
        &enrolled,
        payer_pubkey,
        None,
        3,
        Pubkey::new_from_array([0u8; 32]),
    )
    .await;

    let program_config_data = fetch_program_config(&mut context, enrolled.program_config_pda).await;
    let program_config = ProgramConfig::load(&program_config_data).unwrap();

    assert_eq!(program_config.signer_configs[3].signer, [0u8; 32].into());
}

#[tokio::test]
async fn init_enroll_and_delete_market_config() {
    let InitializedProgramContext {
        mut context,
        program_id,
        config_pda,
    } = setup_initialized_program().await;
    let payer_pubkey = context.payer.pubkey();

    let enrolled = enroll_program(
        &mut context,
        program_id,
        config_pda,
        payer_pubkey,
        None,
        MarketUpdateMode::SingleMarket,
    )
    .await;
    let index = 4u8;
    let mut instruction_data = vec![2u8, index];
    instruction_data.extend_from_slice(&[9u8; 32]);

    let instruction = Instruction::new_with_bytes(
        program_id,
        &instruction_data,
        vec![
            AccountMeta::new_readonly(payer_pubkey, true),
            AccountMeta::new(enrolled.program_config_pda, false),
            AccountMeta::new_readonly(config_pda, false),
            AccountMeta::new_readonly(enrolled.target_program_id, false),
            AccountMeta::new_readonly(enrolled.executable_data_key, false),
        ],
    );

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer_pubkey),
        &[&context.payer],
        context.last_blockhash,
    );

    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    let mut delete_data = vec![2u8, index];
    delete_data.extend_from_slice(&[0u8; 32]);
    let delete_instruction = Instruction::new_with_bytes(
        program_id,
        &delete_data,
        vec![
            AccountMeta::new_readonly(payer_pubkey, true),
            AccountMeta::new(enrolled.program_config_pda, false),
            AccountMeta::new_readonly(config_pda, false),
            AccountMeta::new_readonly(enrolled.target_program_id, false),
            AccountMeta::new_readonly(enrolled.executable_data_key, false),
        ],
    );

    let delete_transaction = Transaction::new_signed_with_payer(
        &[delete_instruction],
        Some(&payer_pubkey),
        &[&context.payer],
        context.last_blockhash,
    );

    context
        .banks_client
        .process_transaction(delete_transaction)
        .await
        .unwrap();

    let program_config_data = fetch_program_config(&mut context, enrolled.program_config_pda).await;
    let program_config = ProgramConfig::load(&program_config_data).unwrap();

    assert_eq!(
        program_config.market_configs[index as usize].writable_account,
        [0u8; 32].into()
    );
    assert_eq!(
        program_config.market_configs[index as usize].market_id,
        [0u8; 32].into()
    );
}

#[tokio::test]
async fn init_enroll_assign_delegate_and_unenroll_as_override_authority() {
    let InitializedProgramContext {
        mut context,
        program_id,
        config_pda,
    } = setup_initialized_program().await;
    let payer_pubkey = context.payer.pubkey();
    let override_authority = Keypair::new();
    let delegate_authority = Keypair::new();

    let blank_signer_account = Account {
        lamports: 10_000_000_000,
        data: vec![],
        owner: system_program::id(),
        executable: false,
        rent_epoch: 0,
    };
    context.set_account(
        &override_authority.pubkey(),
        &blank_signer_account.clone().into(),
    );
    context.set_account(&delegate_authority.pubkey(), &blank_signer_account.into());
    store_override_authority(&mut context, config_pda, override_authority.pubkey()).await;

    let enrolled = enroll_program(
        &mut context,
        program_id,
        config_pda,
        payer_pubkey,
        None,
        MarketUpdateMode::SingleMarket,
    )
    .await;
    assign_delegate_authority(
        &mut context,
        program_id,
        config_pda,
        &enrolled,
        override_authority.pubkey(),
        Some(&override_authority),
        delegate_authority.pubkey(),
    )
    .await;

    let unenroll_instruction = Instruction::new_with_bytes(
        program_id,
        &[3u8],
        vec![
            AccountMeta::new_readonly(override_authority.pubkey(), true),
            AccountMeta::new_readonly(config_pda, false),
            AccountMeta::new_readonly(enrolled.target_program_id, false),
            AccountMeta::new_readonly(enrolled.executable_data_key, false),
            AccountMeta::new(enrolled.program_config_pda, false),
        ],
    );

    let unenroll_transaction = Transaction::new_signed_with_payer(
        &[unenroll_instruction],
        Some(&context.payer.pubkey()),
        &[&context.payer, &override_authority],
        context.last_blockhash,
    );

    context
        .banks_client
        .process_transaction(unenroll_transaction)
        .await
        .unwrap();

    let program_config_data = fetch_program_config(&mut context, enrolled.program_config_pda).await;
    let program_config = ProgramConfig::load(&program_config_data).unwrap();
    assert_eq!(
        program_config.delegate_authority,
        delegate_authority.pubkey().to_bytes().into()
    );
    assert_eq!(program_config.status, ProgramStatus::Unenroll);
}
