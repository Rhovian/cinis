extern crate std;
use {
    crate::idl_client::{
        AcceptInstruction, CancelInstruction, CreateInstruction, InitializeConfigInstruction,
        ResolveInstruction, UpdateConfigInstruction,
    },
    alloc::{vec, vec::Vec},
    mollusk_svm::{program::keyed_account_for_system_program, Mollusk},
    mollusk_svm_programs_token::associated_token,
    solana_account::Account,
    solana_address::Address,
    solana_instruction::Instruction,
    solana_program_pack::Pack,
    spl_associated_token_account_interface::address::get_associated_token_address_with_program_id,
    spl_token_interface::state::{Account as TokenAccount, Mint},
    std::println,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn with_signers(mut ix: Instruction, indices: &[usize]) -> Instruction {
    for &i in indices {
        ix.accounts[i].is_signer = true;
    }
    ix
}

fn setup() -> Mollusk {
    let mut mollusk = Mollusk::new(&crate::ID, "../../target/deploy/cinis");
    mollusk_svm_programs_token::token::add_program(&mut mollusk);
    associated_token::add_program(&mut mollusk);
    mollusk
}

fn pack_token(mint: Address, owner: Address, amount: u64) -> Vec<u8> {
    let token = TokenAccount {
        mint,
        owner,
        amount,
        delegate: None.into(),
        state: spl_token_interface::state::AccountState::Initialized,
        is_native: None.into(),
        delegated_amount: 0,
        close_authority: None.into(),
    };
    let mut data = vec![0u8; TokenAccount::LEN];
    Pack::pack(token, &mut data).unwrap();
    data
}

fn pack_mint(authority: Address, decimals: u8) -> Vec<u8> {
    let mint = Mint {
        mint_authority: Some(authority).into(),
        supply: 1_000_000_000,
        decimals,
        is_initialized: true,
        freeze_authority: None.into(),
    };
    let mut data = vec![0u8; Mint::LEN];
    Pack::pack(mint, &mut data).unwrap();
    data
}

/// Config layout: disc(1) + admin(32) + treasury(32) + fee_bps(2) + bump(1) = 68
fn build_config_data(admin: Address, treasury: Address, fee_bps: u16, bump: u8) -> Vec<u8> {
    let mut data = vec![0u8; 68];
    data[0] = 1; // Config discriminator
    data[1..33].copy_from_slice(admin.as_ref());
    data[33..65].copy_from_slice(treasury.as_ref());
    data[65..67].copy_from_slice(&fee_bps.to_le_bytes());
    data[67] = bump;
    data
}

/// Challenger layout: disc(1) + next_id(8) + bump(1) = 10
fn build_challenger_data(next_id: u64, bump: u8) -> Vec<u8> {
    let mut data = vec![0u8; 10];
    data[0] = 2; // Challenger discriminator
    data[1..9].copy_from_slice(&next_id.to_le_bytes());
    data[9] = bump;
    data
}

/// Duel layout: disc(1) + challenger(32) + opponent(32) + mint(32) + stake(8)
///   + expiry(8) + duel_id(8) + status(1) + bump(1) = 123
fn build_duel_data(
    challenger: Address,
    opponent: Address,
    mint: Address,
    stake: u64,
    expiry: i64,
    duel_id: u64,
    status: u8,
    bump: u8,
) -> Vec<u8> {
    let mut data = vec![0u8; 123];
    data[0] = 3; // Duel discriminator
    data[1..33].copy_from_slice(challenger.as_ref());
    data[33..65].copy_from_slice(opponent.as_ref());
    data[65..97].copy_from_slice(mint.as_ref());
    data[97..105].copy_from_slice(&stake.to_le_bytes());
    data[105..113].copy_from_slice(&expiry.to_le_bytes());
    data[113..121].copy_from_slice(&duel_id.to_le_bytes());
    data[121] = status;
    data[122] = bump;
    data
}

fn config_pda() -> (Address, u8) {
    Address::find_program_address(&[b"config"], &crate::ID)
}

fn challenger_pda(challenger: Address) -> (Address, u8) {
    Address::find_program_address(&[b"challenger", challenger.as_ref()], &crate::ID)
}

fn duel_pda(challenger: Address, duel_id: u64) -> (Address, u8) {
    Address::find_program_address(
        &[b"duel", challenger.as_ref(), &duel_id.to_le_bytes()],
        &crate::ID,
    )
}

fn ata(wallet: Address, mint: Address, token_program: Address) -> Address {
    use solana_pubkey::Pubkey;
    let w = Pubkey::new_from_array(*wallet.as_array());
    let m = Pubkey::new_from_array(*mint.as_array());
    let tp = Pubkey::new_from_array(*token_program.as_array());
    let pk = get_associated_token_address_with_program_id(&w, &m, &tp);
    Address::new_from_array(pk.to_bytes())
}

// ---------------------------------------------------------------------------
// Test: initialize_config
// ---------------------------------------------------------------------------

#[test]
fn test_initialize_config() {
    let mollusk = setup();
    let (_, system_program_account) = keyed_account_for_system_program();
    let (system_program, _) = keyed_account_for_system_program();
    let admin = Address::new_unique();
    let admin_account = Account::new(1_000_000_000, 0, &system_program);
    let treasury = Address::new_unique();
    let treasury_account = Account::default();
    let (config, config_bump) = config_pda();
    let config_account = Account::default();
    let (rent, rent_account) = mollusk.sysvars.keyed_account_for_rent_sysvar();

    let fee_bps = 250u16;

    let instruction = with_signers(
        InitializeConfigInstruction {
            admin,
            config,
            treasury,
            rent,
            system_program,
            fee_bps,
        }
        .into(),
        &[],
    );

    let result = mollusk.process_instruction(
        &instruction,
        &[
            (admin, admin_account),
            (config, config_account),
            (treasury, treasury_account),
            (rent, rent_account),
            (system_program, system_program_account),
        ],
    );

    assert!(
        result.program_result.is_ok(),
        "initialize_config failed: {:?}, raw: {:?}",
        result.program_result,
        result.raw_result
    );

    let data = &result.resulting_accounts[1].1.data;
    assert_eq!(data.len(), 68, "config data length");
    assert_eq!(data[0], 1, "discriminator");
    assert_eq!(&data[1..33], admin.as_ref(), "admin");
    assert_eq!(&data[33..65], treasury.as_ref(), "treasury");
    assert_eq!(&data[65..67], &fee_bps.to_le_bytes(), "fee_bps");
    assert_eq!(data[67], config_bump, "bump");

    println!(
        "  INITIALIZE_CONFIG CU: {}",
        result.compute_units_consumed
    );
}

// ---------------------------------------------------------------------------
// Test: update_config
// ---------------------------------------------------------------------------

#[test]
fn test_update_config() {
    let mollusk = setup();
    let (system_program, _) = keyed_account_for_system_program();

    let admin = Address::new_unique();
    let admin_account = Account::new(1_000_000_000, 0, &system_program);
    let treasury = Address::new_unique();
    let new_treasury = Address::new_unique();
    let new_treasury_account = Account::default();

    let (config, config_bump) = config_pda();
    let config_account = Account {
        lamports: 2_000_000,
        data: build_config_data(admin, treasury, 250, config_bump),
        owner: crate::ID,
        executable: false,
        rent_epoch: 0,
    };

    let new_fee_bps = 500u16;
    let instruction = with_signers(
        UpdateConfigInstruction {
            admin,
            config,
            new_treasury,
            fee_bps: new_fee_bps,
        }
        .into(),
        &[],
    );

    let result = mollusk.process_instruction(
        &instruction,
        &[
            (admin, admin_account),
            (config, config_account),
            (new_treasury, new_treasury_account),
        ],
    );

    assert!(
        result.program_result.is_ok(),
        "update_config failed: {:?}",
        result.program_result
    );

    let data = &result.resulting_accounts[1].1.data;
    assert_eq!(&data[1..33], admin.as_ref(), "admin preserved");
    assert_eq!(&data[33..65], new_treasury.as_ref(), "treasury rotated");
    assert_eq!(&data[65..67], &new_fee_bps.to_le_bytes(), "fee_bps updated");
    assert_eq!(data[67], config_bump, "bump preserved");
}

#[test]
fn test_update_config_wrong_admin_fails() {
    let mollusk = setup();
    let (system_program, _) = keyed_account_for_system_program();

    let admin = Address::new_unique();
    let rando = Address::new_unique();
    let rando_account = Account::new(1_000_000_000, 0, &system_program);
    let treasury = Address::new_unique();
    let new_treasury = Address::new_unique();

    let (config, config_bump) = config_pda();
    let config_account = Account {
        lamports: 2_000_000,
        data: build_config_data(admin, treasury, 250, config_bump),
        owner: crate::ID,
        executable: false,
        rent_epoch: 0,
    };

    let instruction = with_signers(
        UpdateConfigInstruction {
            admin: rando,
            config,
            new_treasury,
            fee_bps: 500,
        }
        .into(),
        &[],
    );

    let result = mollusk.process_instruction(
        &instruction,
        &[
            (rando, rando_account),
            (config, config_account),
            (new_treasury, Account::default()),
        ],
    );

    assert!(
        result.program_result.is_err(),
        "update_config by non-admin should fail"
    );
}

// ---------------------------------------------------------------------------
// Test: create
// ---------------------------------------------------------------------------

#[test]
fn test_create() {
    let mollusk = setup();
    let (token_program_addr, token_program_account) =
        mollusk_svm_programs_token::token::keyed_account();
    let (ata_program_addr, ata_program_account) = associated_token::keyed_account();
    let (system_program, system_program_account) = keyed_account_for_system_program();

    let admin = Address::new_unique();
    let treasury = Address::new_unique();
    let (config, config_bump) = config_pda();
    let config_account = Account {
        lamports: 2_000_000,
        data: build_config_data(admin, treasury, 250, config_bump),
        owner: crate::ID,
        executable: false,
        rent_epoch: 0,
    };

    let challenger = Address::new_unique();
    let challenger_account = Account::new(1_000_000_000, 0, &system_program);

    let (challenger_state, _) = challenger_pda(challenger);
    let challenger_state_account = Account::new(0, 0, &system_program);

    let duel_id = 0u64;
    let (duel, _duel_bump) = duel_pda(challenger, duel_id);
    let duel_account = Account::default();

    let mint = Address::new_unique();
    let mint_account = Account {
        lamports: 1_000_000,
        data: pack_mint(challenger, 9),
        owner: token_program_addr,
        executable: false,
        rent_epoch: 0,
    };

    let challenger_ta = Address::new_unique();
    let challenger_ta_account = Account {
        lamports: 2_039_280,
        data: pack_token(mint, challenger, 1_000_000),
        owner: token_program_addr,
        executable: false,
        rent_epoch: 0,
    };

    let vault = ata(duel, mint, token_program_addr);
    let vault_account = Account::new(0, 0, &system_program);

    let (rent, rent_account) = mollusk.sysvars.keyed_account_for_rent_sysvar();

    let stake = 5_000u64;
    let expiry = 1_700_000_000i64;

    let instruction = with_signers(
        CreateInstruction {
            challenger,
            config,
            challenger_state,
            duel,
            mint,
            challenger_ta,
            vault,
            rent,
            token_program: token_program_addr,
            associated_token_program: ata_program_addr,
            system_program,
            duel_id,
            stake,
            expiry,
        }
        .into(),
        &[],
    );

    let result = mollusk.process_instruction(
        &instruction,
        &[
            (challenger, challenger_account),
            (config, config_account),
            (challenger_state, challenger_state_account),
            (duel, duel_account),
            (mint, mint_account),
            (challenger_ta, challenger_ta_account),
            (vault, vault_account),
            (rent, rent_account),
            (token_program_addr, token_program_account),
            (ata_program_addr, ata_program_account),
            (system_program, system_program_account),
        ],
    );

    assert!(
        result.program_result.is_ok(),
        "create failed: {:?}, raw: {:?}",
        result.program_result,
        result.raw_result
    );

    // Challenger PDA: next_id bumped to 1
    let challenger_state_data = &result.resulting_accounts[2].1.data;
    assert_eq!(challenger_state_data.len(), 10, "challenger data length");
    assert_eq!(challenger_state_data[0], 2, "challenger discriminator");
    assert_eq!(
        &challenger_state_data[1..9],
        &1u64.to_le_bytes(),
        "next_id bumped"
    );

    // Duel PDA
    let duel_data = &result.resulting_accounts[3].1.data;
    assert_eq!(duel_data.len(), 123, "duel data length");
    assert_eq!(duel_data[0], 3, "duel discriminator");
    assert_eq!(&duel_data[1..33], challenger.as_ref(), "challenger");
    assert_eq!(&duel_data[33..65], &[0u8; 32], "opponent (empty)");
    assert_eq!(&duel_data[65..97], mint.as_ref(), "mint");
    assert_eq!(&duel_data[97..105], &stake.to_le_bytes(), "stake");
    assert_eq!(&duel_data[105..113], &expiry.to_le_bytes(), "expiry");
    assert_eq!(&duel_data[113..121], &duel_id.to_le_bytes(), "duel_id");
    assert_eq!(duel_data[121], 0, "status (pending)");

    // Vault: stake deposited
    let vault_data = &result.resulting_accounts[6].1.data;
    let vault_token: TokenAccount = Pack::unpack(vault_data).unwrap();
    assert_eq!(vault_token.amount, stake, "vault balance");
    assert_eq!(vault_token.owner, duel, "vault authority = duel");

    println!("  CREATE CU: {}", result.compute_units_consumed);
}

#[test]
fn test_create_second_duel_increments_tip() {
    let mollusk = setup();
    let (token_program_addr, token_program_account) =
        mollusk_svm_programs_token::token::keyed_account();
    let (ata_program_addr, ata_program_account) = associated_token::keyed_account();
    let (system_program, system_program_account) = keyed_account_for_system_program();

    let admin = Address::new_unique();
    let treasury = Address::new_unique();
    let (config, config_bump) = config_pda();
    let config_account = Account {
        lamports: 2_000_000,
        data: build_config_data(admin, treasury, 250, config_bump),
        owner: crate::ID,
        executable: false,
        rent_epoch: 0,
    };

    let challenger = Address::new_unique();
    let challenger_account = Account::new(1_000_000_000, 0, &system_program);

    // Challenger PDA exists with next_id = 1 (duel 0 already created)
    let (challenger_state, chal_bump) = challenger_pda(challenger);
    let challenger_state_account = Account {
        lamports: 2_000_000,
        data: build_challenger_data(1, chal_bump),
        owner: crate::ID,
        executable: false,
        rent_epoch: 0,
    };

    let duel_id = 1u64;
    let (duel, _) = duel_pda(challenger, duel_id);
    let duel_account = Account::default();

    let mint = Address::new_unique();
    let mint_account = Account {
        lamports: 1_000_000,
        data: pack_mint(challenger, 9),
        owner: token_program_addr,
        executable: false,
        rent_epoch: 0,
    };

    let challenger_ta = Address::new_unique();
    let challenger_ta_account = Account {
        lamports: 2_039_280,
        data: pack_token(mint, challenger, 1_000_000),
        owner: token_program_addr,
        executable: false,
        rent_epoch: 0,
    };

    let vault = ata(duel, mint, token_program_addr);
    let vault_account = Account::new(0, 0, &system_program);

    let (rent, rent_account) = mollusk.sysvars.keyed_account_for_rent_sysvar();

    let instruction = with_signers(
        CreateInstruction {
            challenger,
            config,
            challenger_state,
            duel,
            mint,
            challenger_ta,
            vault,
            rent,
            token_program: token_program_addr,
            associated_token_program: ata_program_addr,
            system_program,
            duel_id,
            stake: 5_000,
            expiry: 1_700_000_000,
        }
        .into(),
        &[],
    );

    let result = mollusk.process_instruction(
        &instruction,
        &[
            (challenger, challenger_account),
            (config, config_account),
            (challenger_state, challenger_state_account),
            (duel, duel_account),
            (mint, mint_account),
            (challenger_ta, challenger_ta_account),
            (vault, vault_account),
            (rent, rent_account),
            (token_program_addr, token_program_account),
            (ata_program_addr, ata_program_account),
            (system_program, system_program_account),
        ],
    );

    assert!(
        result.program_result.is_ok(),
        "create duel 1 failed: {:?}, raw: {:?}",
        result.program_result,
        result.raw_result
    );

    // next_id now 2
    let cs_data = &result.resulting_accounts[2].1.data;
    assert_eq!(&cs_data[1..9], &2u64.to_le_bytes(), "next_id = 2");
}

#[test]
fn test_create_wrong_duel_id_fails() {
    let mollusk = setup();
    let (token_program_addr, token_program_account) =
        mollusk_svm_programs_token::token::keyed_account();
    let (ata_program_addr, ata_program_account) = associated_token::keyed_account();
    let (system_program, system_program_account) = keyed_account_for_system_program();

    let admin = Address::new_unique();
    let treasury = Address::new_unique();
    let (config, config_bump) = config_pda();
    let config_account = Account {
        lamports: 2_000_000,
        data: build_config_data(admin, treasury, 250, config_bump),
        owner: crate::ID,
        executable: false,
        rent_epoch: 0,
    };

    let challenger = Address::new_unique();
    let challenger_account = Account::new(1_000_000_000, 0, &system_program);

    // Tip is at 0 (fresh), but client attempts duel_id = 5
    let (challenger_state, _) = challenger_pda(challenger);
    let challenger_state_account = Account::new(0, 0, &system_program);

    let duel_id = 5u64;
    let (duel, _) = duel_pda(challenger, duel_id);
    let duel_account = Account::default();

    let mint = Address::new_unique();
    let mint_account = Account {
        lamports: 1_000_000,
        data: pack_mint(challenger, 9),
        owner: token_program_addr,
        executable: false,
        rent_epoch: 0,
    };

    let challenger_ta = Address::new_unique();
    let challenger_ta_account = Account {
        lamports: 2_039_280,
        data: pack_token(mint, challenger, 1_000_000),
        owner: token_program_addr,
        executable: false,
        rent_epoch: 0,
    };

    let vault = ata(duel, mint, token_program_addr);
    let vault_account = Account::new(0, 0, &system_program);

    let (rent, rent_account) = mollusk.sysvars.keyed_account_for_rent_sysvar();

    let instruction = with_signers(
        CreateInstruction {
            challenger,
            config,
            challenger_state,
            duel,
            mint,
            challenger_ta,
            vault,
            rent,
            token_program: token_program_addr,
            associated_token_program: ata_program_addr,
            system_program,
            duel_id,
            stake: 5_000,
            expiry: 1_700_000_000,
        }
        .into(),
        &[],
    );

    let result = mollusk.process_instruction(
        &instruction,
        &[
            (challenger, challenger_account),
            (config, config_account),
            (challenger_state, challenger_state_account),
            (duel, duel_account),
            (mint, mint_account),
            (challenger_ta, challenger_ta_account),
            (vault, vault_account),
            (rent, rent_account),
            (token_program_addr, token_program_account),
            (ata_program_addr, ata_program_account),
            (system_program, system_program_account),
        ],
    );

    assert!(
        result.program_result.is_err(),
        "wrong duel_id should fail"
    );
}

// ---------------------------------------------------------------------------
// Test: accept
// ---------------------------------------------------------------------------

#[test]
fn test_accept() {
    let mollusk = setup();
    let (token_program_addr, token_program_account) =
        mollusk_svm_programs_token::token::keyed_account();
    let (system_program, system_program_account) = keyed_account_for_system_program();

    let challenger = Address::new_unique();
    let challenger_account = Account::new(1_000_000, 0, &system_program);
    let opponent = Address::new_unique();
    let opponent_account = Account::new(1_000_000_000, 0, &system_program);

    let mint = Address::new_unique();
    let mint_account = Account {
        lamports: 1_000_000,
        data: pack_mint(challenger, 9),
        owner: token_program_addr,
        executable: false,
        rent_epoch: 0,
    };

    let stake = 5_000u64;
    let duel_id = 0u64;
    let (duel, duel_bump) = duel_pda(challenger, duel_id);
    let duel_account = Account {
        lamports: 2_000_000,
        data: build_duel_data(
            challenger,
            Address::default(),
            mint,
            stake,
            1_700_000_000,
            duel_id,
            0,
            duel_bump,
        ),
        owner: crate::ID,
        executable: false,
        rent_epoch: 0,
    };

    let opponent_ta = Address::new_unique();
    let opponent_ta_account = Account {
        lamports: 1_000_000,
        data: pack_token(mint, opponent, 100_000),
        owner: token_program_addr,
        executable: false,
        rent_epoch: 0,
    };

    let vault = ata(duel, mint, token_program_addr);
    let vault_account = Account {
        lamports: 2_039_280,
        data: pack_token(mint, duel, stake),
        owner: token_program_addr,
        executable: false,
        rent_epoch: 0,
    };

    let _ = challenger_account; // unused now that challenger isn't an account
    let instruction: Instruction = AcceptInstruction {
        opponent,
        duel,
        mint,
        opponent_ta,
        vault,
        token_program: token_program_addr,
        challenger_key: challenger,
        duel_id,
    }
    .into();

    let result = mollusk.process_instruction(
        &instruction,
        &[
            (opponent, opponent_account),
            (duel, duel_account),
            (mint, mint_account),
            (opponent_ta, opponent_ta_account),
            (vault, vault_account),
            (token_program_addr, token_program_account),
            (system_program, system_program_account),
        ],
    );

    assert!(
        result.program_result.is_ok(),
        "accept failed: {:?}, raw: {:?}",
        result.program_result,
        result.raw_result
    );

    let duel_data = &result.resulting_accounts[1].1.data;
    assert_eq!(&duel_data[33..65], opponent.as_ref(), "opponent set");
    assert_eq!(duel_data[121], 1, "status (active)");

    println!("  ACCEPT CU: {}", result.compute_units_consumed);
}

// ---------------------------------------------------------------------------
// Test: resolve
// ---------------------------------------------------------------------------

fn resolve_fixture(
    winner_byte: u8,
) -> (
    Mollusk,
    Instruction,
    Vec<(Address, Account)>,
    Address, // winner wallet
) {
    let mollusk = setup();
    let (token_program_addr, token_program_account) =
        mollusk_svm_programs_token::token::keyed_account();
    let (system_program, system_program_account) = keyed_account_for_system_program();

    let admin = Address::new_unique();
    let admin_account = Account::new(1_000_000_000, 0, &system_program);
    let treasury = Address::new_unique();

    let (config, config_bump) = config_pda();
    let config_account = Account {
        lamports: 2_000_000,
        data: build_config_data(admin, treasury, 250, config_bump),
        owner: crate::ID,
        executable: false,
        rent_epoch: 0,
    };

    let challenger = Address::new_unique();
    let opponent = Address::new_unique();
    let winner_wallet = if winner_byte == 0 { challenger } else { opponent };

    let mint = Address::new_unique();
    let mint_account = Account {
        lamports: 1_000_000,
        data: pack_mint(admin, 9),
        owner: token_program_addr,
        executable: false,
        rent_epoch: 0,
    };

    let stake = 5_000u64;
    let duel_id = 0u64;
    let (duel, duel_bump) = duel_pda(challenger, duel_id);
    let duel_account = Account {
        lamports: 2_000_000,
        data: build_duel_data(
            challenger,
            opponent,
            mint,
            stake,
            1_700_000_000,
            duel_id,
            1,
            duel_bump,
        ),
        owner: crate::ID,
        executable: false,
        rent_epoch: 0,
    };

    let vault = ata(duel, mint, token_program_addr);
    let vault_account = Account {
        lamports: 2_039_280,
        data: pack_token(mint, duel, stake * 2),
        owner: token_program_addr,
        executable: false,
        rent_epoch: 0,
    };

    let winner_ta = Address::new_unique();
    let winner_ta_account = Account {
        lamports: 2_039_280,
        data: pack_token(mint, winner_wallet, 0),
        owner: token_program_addr,
        executable: false,
        rent_epoch: 0,
    };

    let treasury_ta = ata(treasury, mint, token_program_addr);
    let treasury_ta_account = Account {
        lamports: 2_039_280,
        data: pack_token(mint, treasury, 0),
        owner: token_program_addr,
        executable: false,
        rent_epoch: 0,
    };

    let (rent, rent_account) = mollusk.sysvars.keyed_account_for_rent_sysvar();

    let instruction: Instruction = ResolveInstruction {
        admin,
        config,
        duel,
        treasury,
        winner_account: winner_wallet,
        mint,
        winner_ta,
        treasury_ta,
        vault,
        rent,
        token_program: token_program_addr,
        system_program,
        challenger_key: challenger,
        duel_id,
        winner: winner_byte,
    }
    .into();

    let accounts: Vec<(Address, Account)> = vec![
        (admin, admin_account),
        (config, config_account),
        (duel, duel_account),
        (treasury, Account::default()),
        (winner_wallet, Account::new(1_000_000, 0, &system_program)),
        (mint, mint_account),
        (winner_ta, winner_ta_account),
        (treasury_ta, treasury_ta_account),
        (vault, vault_account),
        (rent, rent_account),
        (token_program_addr, token_program_account),
        (system_program, system_program_account),
    ];

    (mollusk, instruction, accounts, winner_wallet)
}

#[test]
fn test_resolve_challenger_wins() {
    let (mollusk, instruction, accounts, winner) = resolve_fixture(0);
    let result = mollusk.process_instruction(&instruction, &accounts);

    assert!(
        result.program_result.is_ok(),
        "resolve failed: {:?}, raw: {:?}",
        result.program_result,
        result.raw_result
    );

    // Fee = 2 * 5000 * 250 / 10000 = 250. Winner = 10000 - 250 = 9750.
    let treasury_data = &result.resulting_accounts[7].1.data;
    let treasury_token: TokenAccount = Pack::unpack(treasury_data).unwrap();
    assert_eq!(treasury_token.amount, 250, "fee amount");

    let winner_data = &result.resulting_accounts[6].1.data;
    let winner_token: TokenAccount = Pack::unpack(winner_data).unwrap();
    assert_eq!(winner_token.amount, 9750, "winner payout");
    assert_eq!(winner_token.owner, winner, "winner owner");

    println!("  RESOLVE CU: {}", result.compute_units_consumed);
}

#[test]
fn test_resolve_opponent_wins() {
    let (mollusk, instruction, accounts, winner) = resolve_fixture(1);
    let result = mollusk.process_instruction(&instruction, &accounts);

    assert!(
        result.program_result.is_ok(),
        "resolve opponent failed: {:?}",
        result.program_result
    );

    let winner_data = &result.resulting_accounts[6].1.data;
    let winner_token: TokenAccount = Pack::unpack(winner_data).unwrap();
    assert_eq!(winner_token.amount, 9750, "winner payout");
    assert_eq!(winner_token.owner, winner, "winner owner");
}

// ---------------------------------------------------------------------------
// Test: cancel
// ---------------------------------------------------------------------------

#[test]
fn test_cancel_pending() {
    let mollusk = setup();
    let (token_program_addr, token_program_account) =
        mollusk_svm_programs_token::token::keyed_account();
    let (system_program, system_program_account) = keyed_account_for_system_program();

    let challenger = Address::new_unique();
    let challenger_account = Account::new(1_000_000_000, 0, &system_program);
    let mint = Address::new_unique();
    let mint_account = Account {
        lamports: 1_000_000,
        data: pack_mint(challenger, 9),
        owner: token_program_addr,
        executable: false,
        rent_epoch: 0,
    };
    let stake = 5_000u64;
    let duel_id = 0u64;
    let (duel, duel_bump) = duel_pda(challenger, duel_id);
    let duel_account = Account {
        lamports: 2_000_000,
        data: build_duel_data(
            challenger,
            Address::default(),
            mint,
            stake,
            0,
            duel_id,
            0,
            duel_bump,
        ),
        owner: crate::ID,
        executable: false,
        rent_epoch: 0,
    };

    let vault = ata(duel, mint, token_program_addr);
    let vault_account = Account {
        lamports: 2_039_280,
        data: pack_token(mint, duel, stake),
        owner: token_program_addr,
        executable: false,
        rent_epoch: 0,
    };

    let challenger_ta = Address::new_unique();
    let challenger_ta_account = Account {
        lamports: 2_039_280,
        data: pack_token(mint, challenger, 0),
        owner: token_program_addr,
        executable: false,
        rent_epoch: 0,
    };

    let opponent_ta = Address::new_unique();
    let opponent_ta_account = Account {
        lamports: 2_039_280,
        data: pack_token(mint, Address::default(), 0),
        owner: token_program_addr,
        executable: false,
        rent_epoch: 0,
    };

    let (rent, rent_account) = mollusk.sysvars.keyed_account_for_rent_sysvar();

    let instruction: Instruction = CancelInstruction {
        canceller: challenger,
        duel,
        mint,
        challenger_ta,
        opponent_ta,
        vault,
        rent,
        token_program: token_program_addr,
        system_program,
        challenger_key: challenger,
        duel_id,
    }
    .into();

    let result = mollusk.process_instruction(
        &instruction,
        &[
            (challenger, challenger_account),
            (duel, duel_account),
            (mint, mint_account),
            (challenger_ta, challenger_ta_account),
            (opponent_ta, opponent_ta_account),
            (vault, vault_account),
            (rent, rent_account),
            (token_program_addr, token_program_account),
            (system_program, system_program_account),
        ],
    );

    assert!(
        result.program_result.is_ok(),
        "cancel pending failed: {:?}, raw: {:?}",
        result.program_result,
        result.raw_result
    );

    let ta_data = &result.resulting_accounts[3].1.data;
    let tok: TokenAccount = Pack::unpack(ta_data).unwrap();
    assert_eq!(tok.amount, stake, "refund");

    println!(
        "  CANCEL_PENDING CU: {}",
        result.compute_units_consumed
    );
}

#[test]
fn test_cancel_active_by_opponent() {
    let mollusk = setup();
    let (token_program_addr, token_program_account) =
        mollusk_svm_programs_token::token::keyed_account();
    let (system_program, system_program_account) = keyed_account_for_system_program();

    let challenger = Address::new_unique();
    let opponent = Address::new_unique();
    let opponent_account = Account::new(1_000_000_000, 0, &system_program);
    let mint = Address::new_unique();
    let mint_account = Account {
        lamports: 1_000_000,
        data: pack_mint(challenger, 9),
        owner: token_program_addr,
        executable: false,
        rent_epoch: 0,
    };
    let stake = 5_000u64;
    let duel_id = 0u64;
    let (duel, duel_bump) = duel_pda(challenger, duel_id);
    let duel_account = Account {
        lamports: 2_000_000,
        data: build_duel_data(
            challenger,
            opponent,
            mint,
            stake,
            0,
            duel_id,
            1,
            duel_bump,
        ),
        owner: crate::ID,
        executable: false,
        rent_epoch: 0,
    };

    let vault = ata(duel, mint, token_program_addr);
    let vault_account = Account {
        lamports: 2_039_280,
        data: pack_token(mint, duel, stake * 2),
        owner: token_program_addr,
        executable: false,
        rent_epoch: 0,
    };

    let challenger_ta = Address::new_unique();
    let challenger_ta_account = Account {
        lamports: 2_039_280,
        data: pack_token(mint, challenger, 0),
        owner: token_program_addr,
        executable: false,
        rent_epoch: 0,
    };

    let opponent_ta = Address::new_unique();
    let opponent_ta_account = Account {
        lamports: 2_039_280,
        data: pack_token(mint, opponent, 0),
        owner: token_program_addr,
        executable: false,
        rent_epoch: 0,
    };

    let (rent, rent_account) = mollusk.sysvars.keyed_account_for_rent_sysvar();

    let instruction: Instruction = CancelInstruction {
        canceller: opponent,
        duel,
        mint,
        challenger_ta,
        opponent_ta,
        vault,
        rent,
        token_program: token_program_addr,
        system_program,
        challenger_key: challenger,
        duel_id,
    }
    .into();

    let result = mollusk.process_instruction(
        &instruction,
        &[
            (opponent, opponent_account),
            (duel, duel_account),
            (mint, mint_account),
            (challenger_ta, challenger_ta_account),
            (opponent_ta, opponent_ta_account),
            (vault, vault_account),
            (rent, rent_account),
            (token_program_addr, token_program_account),
            (system_program, system_program_account),
        ],
    );

    assert!(
        result.program_result.is_ok(),
        "cancel active by opponent failed: {:?}, raw: {:?}",
        result.program_result,
        result.raw_result
    );

    let opp_ta_data = &result.resulting_accounts[4].1.data;
    let opp_tok: TokenAccount = Pack::unpack(opp_ta_data).unwrap();
    assert_eq!(opp_tok.amount, stake, "opponent refund");

    let chal_ta_data = &result.resulting_accounts[3].1.data;
    let chal_tok: TokenAccount = Pack::unpack(chal_ta_data).unwrap();
    assert_eq!(chal_tok.amount, stake, "challenger refund");
}

#[test]
fn test_cancel_active_unauthorized_fails() {
    let mollusk = setup();
    let (token_program_addr, token_program_account) =
        mollusk_svm_programs_token::token::keyed_account();
    let (system_program, system_program_account) = keyed_account_for_system_program();

    let challenger = Address::new_unique();
    let opponent = Address::new_unique();
    let rando = Address::new_unique();
    let rando_account = Account::new(1_000_000_000, 0, &system_program);
    let mint = Address::new_unique();
    let mint_account = Account {
        lamports: 1_000_000,
        data: pack_mint(challenger, 9),
        owner: token_program_addr,
        executable: false,
        rent_epoch: 0,
    };
    let stake = 5_000u64;
    let duel_id = 0u64;
    let (duel, duel_bump) = duel_pda(challenger, duel_id);
    let duel_account = Account {
        lamports: 2_000_000,
        data: build_duel_data(
            challenger,
            opponent,
            mint,
            stake,
            0,
            duel_id,
            1,
            duel_bump,
        ),
        owner: crate::ID,
        executable: false,
        rent_epoch: 0,
    };

    let vault = ata(duel, mint, token_program_addr);
    let vault_account = Account {
        lamports: 2_039_280,
        data: pack_token(mint, duel, stake * 2),
        owner: token_program_addr,
        executable: false,
        rent_epoch: 0,
    };

    let challenger_ta = Address::new_unique();
    let challenger_ta_account = Account {
        lamports: 2_039_280,
        data: pack_token(mint, challenger, 0),
        owner: token_program_addr,
        executable: false,
        rent_epoch: 0,
    };
    let opponent_ta = Address::new_unique();
    let opponent_ta_account = Account {
        lamports: 2_039_280,
        data: pack_token(mint, opponent, 0),
        owner: token_program_addr,
        executable: false,
        rent_epoch: 0,
    };

    let (rent, rent_account) = mollusk.sysvars.keyed_account_for_rent_sysvar();

    let instruction: Instruction = CancelInstruction {
        canceller: rando,
        duel,
        mint,
        challenger_ta,
        opponent_ta,
        vault,
        rent,
        token_program: token_program_addr,
        system_program,
        challenger_key: challenger,
        duel_id,
    }
    .into();

    let result = mollusk.process_instruction(
        &instruction,
        &[
            (rando, rando_account),
            (duel, duel_account),
            (mint, mint_account),
            (challenger_ta, challenger_ta_account),
            (opponent_ta, opponent_ta_account),
            (vault, vault_account),
            (rent, rent_account),
            (token_program_addr, token_program_account),
            (system_program, system_program_account),
        ],
    );

    assert!(
        result.program_result.is_err(),
        "cancel by unauthorized party should fail"
    );
}

// ---------------------------------------------------------------------------
// Test: accept past expiry fails
// ---------------------------------------------------------------------------

#[test]
fn test_accept_past_expiry_fails() {
    let mut mollusk = setup();
    let expiry = 1_700_000_000i64;
    mollusk.sysvars.clock.unix_timestamp = expiry + 1;

    let (token_program_addr, token_program_account) =
        mollusk_svm_programs_token::token::keyed_account();
    let (system_program, system_program_account) = keyed_account_for_system_program();

    let challenger = Address::new_unique();
    let challenger_account = Account::new(1_000_000, 0, &system_program);
    let opponent = Address::new_unique();
    let opponent_account = Account::new(1_000_000_000, 0, &system_program);

    let mint = Address::new_unique();
    let mint_account = Account {
        lamports: 1_000_000,
        data: pack_mint(challenger, 9),
        owner: token_program_addr,
        executable: false,
        rent_epoch: 0,
    };

    let stake = 5_000u64;
    let duel_id = 0u64;
    let (duel, duel_bump) = duel_pda(challenger, duel_id);
    let duel_account = Account {
        lamports: 2_000_000,
        data: build_duel_data(
            challenger,
            Address::default(),
            mint,
            stake,
            expiry,
            duel_id,
            0,
            duel_bump,
        ),
        owner: crate::ID,
        executable: false,
        rent_epoch: 0,
    };

    let opponent_ta = Address::new_unique();
    let opponent_ta_account = Account {
        lamports: 1_000_000,
        data: pack_token(mint, opponent, 100_000),
        owner: token_program_addr,
        executable: false,
        rent_epoch: 0,
    };

    let vault = ata(duel, mint, token_program_addr);
    let vault_account = Account {
        lamports: 2_039_280,
        data: pack_token(mint, duel, stake),
        owner: token_program_addr,
        executable: false,
        rent_epoch: 0,
    };

    let _ = challenger_account;
    let instruction: Instruction = AcceptInstruction {
        opponent,
        duel,
        mint,
        opponent_ta,
        vault,
        token_program: token_program_addr,
        challenger_key: challenger,
        duel_id,
    }
    .into();

    let result = mollusk.process_instruction(
        &instruction,
        &[
            (opponent, opponent_account),
            (duel, duel_account),
            (mint, mint_account),
            (opponent_ta, opponent_ta_account),
            (vault, vault_account),
            (token_program_addr, token_program_account),
            (system_program, system_program_account),
        ],
    );

    assert!(
        result.program_result.is_err(),
        "accept past expiry should fail"
    );
}

// ---------------------------------------------------------------------------
// Test: cancel pending past expiry by any party
// ---------------------------------------------------------------------------

#[test]
fn test_cancel_pending_past_expiry_permissionless() {
    let mut mollusk = setup();
    let expiry = 1_700_000_000i64;
    mollusk.sysvars.clock.unix_timestamp = expiry + 1;

    let (token_program_addr, token_program_account) =
        mollusk_svm_programs_token::token::keyed_account();
    let (system_program, system_program_account) = keyed_account_for_system_program();

    let challenger = Address::new_unique();
    let rando = Address::new_unique();
    let rando_account = Account::new(1_000_000_000, 0, &system_program);
    let mint = Address::new_unique();
    let mint_account = Account {
        lamports: 1_000_000,
        data: pack_mint(challenger, 9),
        owner: token_program_addr,
        executable: false,
        rent_epoch: 0,
    };
    let stake = 5_000u64;
    let duel_id = 0u64;
    let (duel, duel_bump) = duel_pda(challenger, duel_id);
    let duel_account = Account {
        lamports: 2_000_000,
        data: build_duel_data(
            challenger,
            Address::default(),
            mint,
            stake,
            expiry,
            duel_id,
            0, // pending
            duel_bump,
        ),
        owner: crate::ID,
        executable: false,
        rent_epoch: 0,
    };

    let vault = ata(duel, mint, token_program_addr);
    let vault_account = Account {
        lamports: 2_039_280,
        data: pack_token(mint, duel, stake),
        owner: token_program_addr,
        executable: false,
        rent_epoch: 0,
    };

    let challenger_ta = Address::new_unique();
    let challenger_ta_account = Account {
        lamports: 2_039_280,
        data: pack_token(mint, challenger, 0),
        owner: token_program_addr,
        executable: false,
        rent_epoch: 0,
    };
    let opponent_ta = Address::new_unique();
    let opponent_ta_account = Account {
        lamports: 2_039_280,
        data: pack_token(mint, Address::default(), 0),
        owner: token_program_addr,
        executable: false,
        rent_epoch: 0,
    };

    let (rent, rent_account) = mollusk.sysvars.keyed_account_for_rent_sysvar();

    let instruction: Instruction = CancelInstruction {
        canceller: rando,
        duel,
        mint,
        challenger_ta,
        opponent_ta,
        vault,
        rent,
        token_program: token_program_addr,
        system_program,
        challenger_key: challenger,
        duel_id,
    }
    .into();

    let result = mollusk.process_instruction(
        &instruction,
        &[
            (rando, rando_account),
            (duel, duel_account),
            (mint, mint_account),
            (challenger_ta, challenger_ta_account),
            (opponent_ta, opponent_ta_account),
            (vault, vault_account),
            (rent, rent_account),
            (token_program_addr, token_program_account),
            (system_program, system_program_account),
        ],
    );

    assert!(
        result.program_result.is_ok(),
        "cancel pending past expiry should succeed: {:?}",
        result.program_result
    );

    let chal_ta_data = &result.resulting_accounts[3].1.data;
    let chal_tok: TokenAccount = Pack::unpack(chal_ta_data).unwrap();
    assert_eq!(chal_tok.amount, stake, "challenger refunded");
}
