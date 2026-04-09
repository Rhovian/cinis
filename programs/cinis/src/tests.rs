extern crate std;
use {
    crate::cpi::{AcceptInstruction, CancelInstruction, CreateInstruction, ResolveInstruction},
    alloc::{vec, vec::Vec},
    mollusk_svm::{program::keyed_account_for_system_program, Mollusk},
    solana_account::Account,
    solana_address::Address,
    solana_instruction::Instruction,
    solana_program_pack::Pack,
    spl_token_interface::state::{Account as TokenAccount, Mint},
    std::println,
};

fn with_signers(mut ix: Instruction, indices: &[usize]) -> Instruction {
    for &i in indices {
        ix.accounts[i].is_signer = true;
    }
    ix
}

fn setup() -> Mollusk {
    let mut mollusk = Mollusk::new(&crate::ID, "../../target/deploy/cinis");
    mollusk_svm_programs_token::token::add_program(&mut mollusk);
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

/// Build raw duel account data (181 bytes).
/// disc(1) + challenger(32) + opponent(32) + mint(32) + authority(32)
/// + fee_account(32) + stake(8) + expiry(8) + fee_bps(2) + status(1) + bump(1)
fn build_duel_data(
    challenger: Address,
    opponent: Address,
    mint: Address,
    authority: Address,
    fee_account: Address,
    stake: u64,
    expiry: i64,
    fee_bps: u16,
    status: u8,
    bump: u8,
) -> Vec<u8> {
    let mut data = vec![0u8; 181];
    data[0] = 1; // Duel discriminator
    data[1..33].copy_from_slice(challenger.as_ref());
    data[33..65].copy_from_slice(opponent.as_ref());
    data[65..97].copy_from_slice(mint.as_ref());
    data[97..129].copy_from_slice(authority.as_ref());
    data[129..161].copy_from_slice(fee_account.as_ref());
    data[161..169].copy_from_slice(&stake.to_le_bytes());
    data[169..177].copy_from_slice(&expiry.to_le_bytes());
    data[177..179].copy_from_slice(&fee_bps.to_le_bytes());
    data[179] = status;
    data[180] = bump;
    data
}

// -------------------------------------------------------------------------
// Test: create duel
// -------------------------------------------------------------------------

#[test]
fn test_create() {
    let mollusk = setup();
    let (token_program, token_program_account) = mollusk_svm_programs_token::token::keyed_account();
    let (system_program, system_program_account) = keyed_account_for_system_program();

    let challenger = Address::new_unique();
    let challenger_account = Account::new(1_000_000_000, 0, &system_program);

    let authority = Address::new_unique();
    let authority_account = Account::default();

    let (duel, duel_bump) =
        Address::find_program_address(&[b"duel", challenger.as_ref()], &crate::ID);
    let duel_account = Account::default();

    let mint = Address::new_unique();
    let mint_account = Account {
        lamports: 1_000_000,
        data: pack_mint(challenger, 9),
        owner: token_program,
        executable: false,
        rent_epoch: 0,
    };

    let challenger_ta = Address::new_unique();
    let challenger_ta_account = Account {
        lamports: 1_000_000,
        data: pack_token(mint, challenger, 1_000_000),
        owner: token_program,
        executable: false,
        rent_epoch: 0,
    };

    let fee_account = Address::new_unique();
    let fee_account_account = Account {
        lamports: 2_039_280,
        data: pack_token(mint, authority, 0),
        owner: token_program,
        executable: false,
        rent_epoch: 0,
    };

    let vault = Address::new_unique();
    let vault_account = Account::new(0, 0, &system_program);

    let (rent, rent_account) = mollusk.sysvars.keyed_account_for_rent_sysvar();

    let stake = 5000u64;
    let fee_bps = 250u16; // 2.5%
    let expiry = 1_700_000_000i64;

    let instruction = with_signers(
        CreateInstruction {
            challenger,
            duel,
            authority,
            mint,
            challenger_ta,
            fee_account,
            vault,
            rent,
            token_program,
            system_program,
            stake,
            fee_bps,
            expiry,
        }
        .into(),
        &[6], // vault (init_if_needed, uninitialized)
    );

    let result = mollusk.process_instruction(
        &instruction,
        &[
            (challenger, challenger_account),
            (duel, duel_account),
            (authority, authority_account),
            (mint, mint_account),
            (challenger_ta, challenger_ta_account),
            (fee_account, fee_account_account),
            (vault, vault_account),
            (rent, rent_account),
            (token_program, token_program_account),
            (system_program, system_program_account),
        ],
    );

    assert!(
        result.program_result.is_ok(),
        "create failed: {:?}",
        result.program_result
    );

    // Validate duel state
    let duel_data = &result.resulting_accounts[1].1.data;
    assert_eq!(duel_data.len(), 181, "duel data length");
    assert_eq!(duel_data[0], 1, "discriminator");
    assert_eq!(&duel_data[1..33], challenger.as_ref(), "challenger");
    assert_eq!(&duel_data[33..65], &[0u8; 32], "opponent (empty)");
    assert_eq!(&duel_data[65..97], mint.as_ref(), "mint");
    assert_eq!(&duel_data[97..129], authority.as_ref(), "authority");
    assert_eq!(&duel_data[129..161], fee_account.as_ref(), "fee_account");
    assert_eq!(&duel_data[161..169], &stake.to_le_bytes(), "stake");
    assert_eq!(&duel_data[169..177], &expiry.to_le_bytes(), "expiry");
    assert_eq!(&duel_data[177..179], &fee_bps.to_le_bytes(), "fee_bps");
    assert_eq!(duel_data[179], 0, "status (pending)");
    assert_eq!(duel_data[180], duel_bump, "bump");

    println!("\n========================================");
    println!("  CREATE CU: {}", result.compute_units_consumed);
    println!("========================================\n");
}

// -------------------------------------------------------------------------
// Test: accept duel
// -------------------------------------------------------------------------

#[test]
fn test_accept() {
    let mollusk = setup();
    let (token_program, token_program_account) = mollusk_svm_programs_token::token::keyed_account();
    let (system_program, _) = keyed_account_for_system_program();

    let challenger = Address::new_unique();
    let challenger_account = Account::new(1_000_000, 0, &system_program);
    let opponent = Address::new_unique();
    let opponent_account = Account::new(1_000_000_000, 0, &system_program);
    let authority = Address::new_unique();
    let mint = Address::new_unique();
    let fee_account = Address::new_unique();
    let stake = 5000u64;
    let fee_bps = 250u16;

    let (duel, duel_bump) =
        Address::find_program_address(&[b"duel", challenger.as_ref()], &crate::ID);
    let duel_account = Account {
        lamports: 2_000_000,
        data: build_duel_data(
            challenger,
            Address::default(),
            mint,
            authority,
            fee_account,
            stake,
            1_700_000_000,
            fee_bps,
            0, // pending
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
        owner: token_program,
        executable: false,
        rent_epoch: 0,
    };

    let vault = Address::new_unique();
    let vault_account = Account {
        lamports: 2_039_280,
        data: pack_token(mint, duel, stake),
        owner: token_program,
        executable: false,
        rent_epoch: 0,
    };

    let instruction: Instruction = AcceptInstruction {
        opponent,
        duel,
        challenger,
        opponent_ta,
        vault,
        token_program,
    }
    .into();

    let result = mollusk.process_instruction(
        &instruction,
        &[
            (opponent, opponent_account),
            (duel, duel_account),
            (challenger, challenger_account),
            (opponent_ta, opponent_ta_account),
            (vault, vault_account),
            (token_program, token_program_account),
        ],
    );

    assert!(
        result.program_result.is_ok(),
        "accept failed: {:?}",
        result.program_result
    );

    let duel_data = &result.resulting_accounts[1].1.data;
    assert_eq!(&duel_data[33..65], opponent.as_ref(), "opponent set");
    assert_eq!(duel_data[179], 1, "status (active)");

    println!("\n========================================");
    println!("  ACCEPT CU: {}", result.compute_units_consumed);
    println!("========================================\n");
}

// -------------------------------------------------------------------------
// Test: resolve duel (challenger wins)
// -------------------------------------------------------------------------

#[test]
fn test_resolve_challenger_wins() {
    let mollusk = setup();
    let (token_program, token_program_account) = mollusk_svm_programs_token::token::keyed_account();
    let (system_program, system_program_account) = keyed_account_for_system_program();

    let challenger = Address::new_unique();
    let challenger_account = Account::new(1_000_000_000, 0, &system_program);
    let opponent = Address::new_unique();
    let authority = Address::new_unique();
    let authority_account = Account::new(1_000_000_000, 0, &system_program);
    let mint = Address::new_unique();
    let stake = 5000u64;
    let fee_bps = 250u16;

    let mint_account = Account {
        lamports: 1_000_000,
        data: pack_mint(challenger, 9),
        owner: token_program,
        executable: false,
        rent_epoch: 0,
    };

    let fee_account = Address::new_unique();
    let fee_account_account = Account {
        lamports: 2_039_280,
        data: pack_token(mint, authority, 0),
        owner: token_program,
        executable: false,
        rent_epoch: 0,
    };

    let (duel, duel_bump) =
        Address::find_program_address(&[b"duel", challenger.as_ref()], &crate::ID);
    let duel_account = Account {
        lamports: 2_000_000,
        data: build_duel_data(
            challenger,
            opponent,
            mint,
            authority,
            fee_account,
            stake,
            1_700_000_000,
            fee_bps,
            1, // active
            duel_bump,
        ),
        owner: crate::ID,
        executable: false,
        rent_epoch: 0,
    };

    let vault = Address::new_unique();
    let vault_account = Account {
        lamports: 2_039_280,
        data: pack_token(mint, duel, stake * 2),
        owner: token_program,
        executable: false,
        rent_epoch: 0,
    };

    let winner_ta = Address::new_unique();
    let winner_ta_account = Account {
        lamports: 2_039_280,
        data: pack_token(mint, challenger, 0),
        owner: token_program,
        executable: false,
        rent_epoch: 0,
    };

    let (rent, rent_account) = mollusk.sysvars.keyed_account_for_rent_sysvar();

    let instruction: Instruction = ResolveInstruction {
        authority,
        duel,
        winner_account: challenger,
        mint,
        winner_ta,
        fee_account,
        vault,
        rent,
        token_program,
        system_program,
        winner: 0,
    }
    .into();
    println!("resolve metas: {:?}", instruction.accounts);

    let result = mollusk.process_instruction(
        &instruction,
        &[
            (authority, authority_account),
            (duel, duel_account),
            (challenger, challenger_account),
            (mint, mint_account),
            (winner_ta, winner_ta_account),
            (fee_account, fee_account_account),
            (vault, vault_account),
            (rent, rent_account),
            (token_program, token_program_account),
            (system_program, system_program_account),
        ],
    );

    assert!(
        result.program_result.is_ok(),
        "resolve failed: {:?}, raw: {:?}",
        result.program_result,
        result.raw_result
    );

    // Fee = 2 * 5000 * 250 / 10000 = 250
    // Winner gets 10000 - 250 = 9750
    let fee_data = &result.resulting_accounts[5].1.data;
    let fee_token: TokenAccount = Pack::unpack(fee_data).unwrap();
    assert_eq!(fee_token.amount, 250, "fee amount");

    let winner_data = &result.resulting_accounts[4].1.data;
    let winner_token: TokenAccount = Pack::unpack(winner_data).unwrap();
    assert_eq!(winner_token.amount, 9750, "winner payout");
    assert_eq!(winner_token.owner, challenger, "winner owner");

    println!("\n========================================");
    println!("  RESOLVE CU: {}", result.compute_units_consumed);
    println!("========================================\n");
}

// -------------------------------------------------------------------------
// Test: resolve duel (opponent wins)
// -------------------------------------------------------------------------

#[test]
fn test_resolve_opponent_wins() {
    let mollusk = setup();
    let (token_program, token_program_account) = mollusk_svm_programs_token::token::keyed_account();
    let (system_program, system_program_account) = keyed_account_for_system_program();

    let challenger = Address::new_unique();
    let opponent = Address::new_unique();
    let authority = Address::new_unique();
    let authority_account = Account::new(1_000_000_000, 0, &system_program);
    let mint = Address::new_unique();
    let stake = 5_000u64;
    let fee_bps = 250u16;

    let mint_account = Account {
        lamports: 1_000_000,
        data: pack_mint(challenger, 9),
        owner: token_program,
        executable: false,
        rent_epoch: 0,
    };

    let fee_account = Address::new_unique();
    let fee_account_account = Account {
        lamports: 2_039_280,
        data: pack_token(mint, authority, 0),
        owner: token_program,
        executable: false,
        rent_epoch: 0,
    };

    let (duel, duel_bump) =
        Address::find_program_address(&[b"duel", challenger.as_ref()], &crate::ID);
    let duel_account = Account {
        lamports: 2_000_000,
        data: build_duel_data(
            challenger,
            opponent,
            mint,
            authority,
            fee_account,
            stake,
            1_700_000_000,
            fee_bps,
            1, // active
            duel_bump,
        ),
        owner: crate::ID,
        executable: false,
        rent_epoch: 0,
    };

    let vault = Address::new_unique();
    let vault_account = Account {
        lamports: 2_039_280,
        data: pack_token(mint, duel, stake * 2),
        owner: token_program,
        executable: false,
        rent_epoch: 0,
    };

    let winner_ta = Address::new_unique();
    let winner_ta_account = Account {
        lamports: 2_039_280,
        data: pack_token(mint, opponent, 0),
        owner: token_program,
        executable: false,
        rent_epoch: 0,
    };

    let (rent, rent_account) = mollusk.sysvars.keyed_account_for_rent_sysvar();

    let instruction: Instruction = ResolveInstruction {
        authority,
        duel,
        winner_account: opponent,
        mint,
        winner_ta,
        fee_account,
        vault,
        rent,
        token_program,
        system_program,
        winner: 1,
    }
    .into();

    let result = mollusk.process_instruction(
        &instruction,
        &[
            (authority, authority_account),
            (duel, duel_account),
            (opponent, Account::new(1_000_000_000, 0, &system_program)),
            (mint, mint_account),
            (winner_ta, winner_ta_account),
            (fee_account, fee_account_account),
            (vault, vault_account),
            (rent, rent_account),
            (token_program, token_program_account),
            (system_program, system_program_account),
        ],
    );

    assert!(
        result.program_result.is_ok(),
        "resolve opponent win failed: {:?}, raw: {:?}",
        result.program_result,
        result.raw_result
    );

    let fee_data = &result.resulting_accounts[5].1.data;
    let fee_token: TokenAccount = Pack::unpack(fee_data).unwrap();
    assert_eq!(fee_token.amount, 250, "fee amount");

    let winner_data = &result.resulting_accounts[4].1.data;
    let winner_token: TokenAccount = Pack::unpack(winner_data).unwrap();
    assert_eq!(winner_token.amount, 9750, "winner payout");
    assert_eq!(winner_token.owner, opponent, "winner owner");
}

// -------------------------------------------------------------------------
// Test: cancel pending duel (challenger cancels)
// -------------------------------------------------------------------------

#[test]
fn test_cancel_pending() {
    let mollusk = setup();
    let (token_program, token_program_account) = mollusk_svm_programs_token::token::keyed_account();
    let (system_program, system_program_account) = keyed_account_for_system_program();

    let challenger = Address::new_unique();
    let challenger_account = Account::new(1_000_000_000, 0, &system_program);
    let authority = Address::new_unique();
    let mint = Address::new_unique();
    let fee_account = Address::new_unique();
    let stake = 5000u64;

    let mint_account = Account {
        lamports: 1_000_000,
        data: pack_mint(challenger, 9),
        owner: token_program,
        executable: false,
        rent_epoch: 0,
    };

    let (duel, duel_bump) =
        Address::find_program_address(&[b"duel", challenger.as_ref()], &crate::ID);
    let duel_account = Account {
        lamports: 2_000_000,
        data: build_duel_data(
            challenger,
            Address::default(),
            mint,
            authority,
            fee_account,
            stake,
            0,
            250,
            0, // pending
            duel_bump,
        ),
        owner: crate::ID,
        executable: false,
        rent_epoch: 0,
    };

    let vault = Address::new_unique();
    let vault_account = Account {
        lamports: 2_039_280,
        data: pack_token(mint, duel, stake),
        owner: token_program,
        executable: false,
        rent_epoch: 0,
    };

    let challenger_ta = Address::new_unique();
    let challenger_ta_account = Account {
        lamports: 2_039_280,
        data: pack_token(mint, challenger, 0),
        owner: token_program,
        executable: false,
        rent_epoch: 0,
    };

    // opponent_ta unused for pending cancel, but must be passed
    let opponent_ta = Address::new_unique();
    let opponent_ta_account = Account {
        lamports: 2_039_280,
        data: pack_token(mint, Address::default(), 0),
        owner: token_program,
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
        token_program,
        system_program,
    }
    .into();
    println!("cancel metas: {:?}", instruction.accounts);

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
            (token_program, token_program_account),
            (system_program, system_program_account),
        ],
    );

    assert!(
        result.program_result.is_ok(),
        "cancel pending failed: {:?}, raw: {:?}",
        result.program_result,
        result.raw_result
    );

    let challenger_ta_data = &result.resulting_accounts[3].1.data;
    let challenger_token: TokenAccount = Pack::unpack(challenger_ta_data).unwrap();
    assert_eq!(challenger_token.amount, stake, "refund amount");

    println!("\n========================================");
    println!("  CANCEL PENDING CU: {}", result.compute_units_consumed);
    println!("========================================\n");
}

// -------------------------------------------------------------------------
// Test: cancel active duel (opponent cancels, each gets stake back)
// -------------------------------------------------------------------------

#[test]
fn test_cancel_active_by_opponent() {
    let mollusk = setup();
    let (token_program, token_program_account) = mollusk_svm_programs_token::token::keyed_account();
    let (system_program, system_program_account) = keyed_account_for_system_program();

    let challenger = Address::new_unique();
    let challenger_account = Account::new(1_000_000_000, 0, &system_program);
    let opponent = Address::new_unique();
    let opponent_account = Account::new(1_000_000_000, 0, &system_program);
    let authority = Address::new_unique();
    let mint = Address::new_unique();
    let fee_account = Address::new_unique();
    let stake = 5000u64;

    let mint_account = Account {
        lamports: 1_000_000,
        data: pack_mint(challenger, 9),
        owner: token_program,
        executable: false,
        rent_epoch: 0,
    };

    let (duel, duel_bump) =
        Address::find_program_address(&[b"duel", challenger.as_ref()], &crate::ID);
    let duel_account = Account {
        lamports: 2_000_000,
        data: build_duel_data(
            challenger,
            opponent,
            mint,
            authority,
            fee_account,
            stake,
            0,
            250,
            1, // active
            duel_bump,
        ),
        owner: crate::ID,
        executable: false,
        rent_epoch: 0,
    };

    let vault = Address::new_unique();
    let vault_account = Account {
        lamports: 2_039_280,
        data: pack_token(mint, duel, stake * 2),
        owner: token_program,
        executable: false,
        rent_epoch: 0,
    };

    let challenger_ta = Address::new_unique();
    let challenger_ta_account = Account {
        lamports: 2_039_280,
        data: pack_token(mint, challenger, 0),
        owner: token_program,
        executable: false,
        rent_epoch: 0,
    };

    let opponent_ta = Address::new_unique();
    let opponent_ta_account = Account {
        lamports: 2_039_280,
        data: pack_token(mint, opponent, 0),
        owner: token_program,
        executable: false,
        rent_epoch: 0,
    };

    let (rent, rent_account) = mollusk.sysvars.keyed_account_for_rent_sysvar();

    // Opponent signs the cancel
    let instruction: Instruction = CancelInstruction {
        canceller: opponent,
        duel,
        mint,
        challenger_ta,
        opponent_ta,
        vault,
        rent,
        token_program,
        system_program,
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
            (token_program, token_program_account),
            (system_program, system_program_account),
        ],
    );

    assert!(
        result.program_result.is_ok(),
        "cancel active by opponent failed: {:?}, raw: {:?}",
        result.program_result,
        result.raw_result
    );

    // Opponent gets their stake back
    let opponent_ta_data = &result.resulting_accounts[4].1.data;
    let opponent_token: TokenAccount = Pack::unpack(opponent_ta_data).unwrap();
    assert_eq!(opponent_token.amount, stake, "opponent refund");

    // Challenger gets their stake back
    let challenger_ta_data = &result.resulting_accounts[3].1.data;
    let challenger_token: TokenAccount = Pack::unpack(challenger_ta_data).unwrap();
    assert_eq!(challenger_token.amount, stake, "challenger refund");

    println!("\n========================================");
    println!("  CANCEL ACTIVE CU: {}", result.compute_units_consumed);
    println!("========================================\n");
}

// -------------------------------------------------------------------------
// Test: cancel active duel by unauthorized party fails
// -------------------------------------------------------------------------

#[test]
fn test_cancel_active_unauthorized_fails() {
    let mollusk = setup();
    let (token_program, token_program_account) = mollusk_svm_programs_token::token::keyed_account();
    let (system_program, system_program_account) = keyed_account_for_system_program();

    let challenger = Address::new_unique();
    let challenger_account = Account::new(1_000_000_000, 0, &system_program);
    let opponent = Address::new_unique();
    let authority = Address::new_unique();
    let rando = Address::new_unique();
    let rando_account = Account::new(1_000_000_000, 0, &system_program);
    let mint = Address::new_unique();
    let fee_account = Address::new_unique();
    let stake = 5000u64;

    let mint_account = Account {
        lamports: 1_000_000,
        data: pack_mint(challenger, 9),
        owner: token_program,
        executable: false,
        rent_epoch: 0,
    };

    let (duel, duel_bump) =
        Address::find_program_address(&[b"duel", challenger.as_ref()], &crate::ID);
    let duel_account = Account {
        lamports: 2_000_000,
        data: build_duel_data(
            challenger,
            opponent,
            mint,
            authority,
            fee_account,
            stake,
            0,
            250,
            1, // active
            duel_bump,
        ),
        owner: crate::ID,
        executable: false,
        rent_epoch: 0,
    };

    let vault = Address::new_unique();
    let vault_account = Account {
        lamports: 2_039_280,
        data: pack_token(mint, duel, stake * 2),
        owner: token_program,
        executable: false,
        rent_epoch: 0,
    };

    let challenger_ta = Address::new_unique();
    let challenger_ta_account = Account {
        lamports: 2_039_280,
        data: pack_token(mint, challenger, 0),
        owner: token_program,
        executable: false,
        rent_epoch: 0,
    };

    let opponent_ta = Address::new_unique();
    let opponent_ta_account = Account {
        lamports: 2_039_280,
        data: pack_token(mint, opponent, 0),
        owner: token_program,
        executable: false,
        rent_epoch: 0,
    };

    let (rent, rent_account) = mollusk.sysvars.keyed_account_for_rent_sysvar();

    // Random party tries to cancel
    let instruction: Instruction = CancelInstruction {
        canceller: rando,
        duel,
        mint,
        challenger_ta,
        opponent_ta,
        vault,
        rent,
        token_program,
        system_program,
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
            (token_program, token_program_account),
            (system_program, system_program_account),
        ],
    );

    assert!(
        result.program_result.is_err(),
        "cancel by unauthorized party should fail"
    );
    println!("  cancel by unauthorized party rejected: OK");
}
