//! Off-chain client for the cinis duel wager protocol.
//!
//! This crate has no dependency on Quasar — it is intended for downstream
//! consumers (CLIs, bots, indexers) that need to construct `cinis`
//! instructions and derive its protocol PDAs without pulling the on-chain
//! framework into their build graph.
//!
//! Associated token accounts are not derived here — callers pass `mint` and
//! `token_program` explicitly, since which token program is in use is their
//! choice.

use solana_address::{declare_id, Address};
use solana_instruction::{AccountMeta, Instruction};

declare_id!("6gFMC9Rw5DjqyLQBY4QXRcvFHfg8bPQABfhHV2nuyRF");

// ---------------------------------------------------------------------------
// PDA helpers
// ---------------------------------------------------------------------------

/// Derive the singleton `Config` PDA. Seeds: `[b"config"]`.
pub fn config_pda() -> (Address, u8) {
    Address::find_program_address(&[b"config"], &ID)
}

/// Derive the per-challenger tip-tracker PDA. Seeds: `[b"challenger", challenger]`.
pub fn challenger_pda(challenger: &Address) -> (Address, u8) {
    Address::find_program_address(&[b"challenger", challenger.as_ref()], &ID)
}

/// Derive a duel PDA. Seeds: `[b"duel", challenger, duel_id.to_le_bytes()]`.
pub fn duel_pda(challenger: &Address, duel_id: u64) -> (Address, u8) {
    Address::find_program_address(
        &[b"duel", challenger.as_ref(), &duel_id.to_le_bytes()],
        &ID,
    )
}

// ---------------------------------------------------------------------------
// initialize_config (discriminator = 0)
// ---------------------------------------------------------------------------

pub struct InitializeConfigInstruction {
    pub admin: Address,
    pub config: Address,
    pub treasury: Address,
    pub rent: Address,
    pub system_program: Address,
    pub fee_bps: u16,
}

impl From<InitializeConfigInstruction> for Instruction {
    fn from(ix: InitializeConfigInstruction) -> Instruction {
        let accounts = vec![
            AccountMeta::new(ix.admin, true),
            AccountMeta::new(ix.config, false),
            AccountMeta::new_readonly(ix.treasury, false),
            AccountMeta::new_readonly(ix.rent, false),
            AccountMeta::new_readonly(ix.system_program, false),
        ];
        let mut data = vec![0u8];
        data.extend_from_slice(&ix.fee_bps.to_le_bytes());
        Instruction {
            program_id: ID,
            accounts,
            data,
        }
    }
}

// ---------------------------------------------------------------------------
// update_config (discriminator = 1)
// ---------------------------------------------------------------------------

pub struct UpdateConfigInstruction {
    pub admin: Address,
    pub config: Address,
    pub new_treasury: Address,
    pub fee_bps: u16,
}

impl From<UpdateConfigInstruction> for Instruction {
    fn from(ix: UpdateConfigInstruction) -> Instruction {
        let accounts = vec![
            AccountMeta::new_readonly(ix.admin, true),
            AccountMeta::new(ix.config, false),
            AccountMeta::new_readonly(ix.new_treasury, false),
        ];
        let mut data = vec![1u8];
        data.extend_from_slice(&ix.fee_bps.to_le_bytes());
        Instruction {
            program_id: ID,
            accounts,
            data,
        }
    }
}

// ---------------------------------------------------------------------------
// create (discriminator = 2)
// ---------------------------------------------------------------------------

pub struct CreateInstruction {
    pub challenger: Address,
    pub config: Address,
    pub challenger_state: Address,
    pub duel: Address,
    pub mint: Address,
    pub challenger_ta: Address,
    pub vault: Address,
    pub rent: Address,
    pub token_program: Address,
    pub associated_token_program: Address,
    pub system_program: Address,
    pub duel_id: u64,
    pub stake: u64,
    pub expiry: i64,
}

impl From<CreateInstruction> for Instruction {
    fn from(ix: CreateInstruction) -> Instruction {
        let accounts = vec![
            AccountMeta::new(ix.challenger, true),
            AccountMeta::new_readonly(ix.config, false),
            AccountMeta::new(ix.challenger_state, false),
            AccountMeta::new(ix.duel, false),
            AccountMeta::new_readonly(ix.mint, false),
            AccountMeta::new(ix.challenger_ta, false),
            AccountMeta::new(ix.vault, false),
            AccountMeta::new_readonly(ix.rent, false),
            AccountMeta::new_readonly(ix.token_program, false),
            AccountMeta::new_readonly(ix.associated_token_program, false),
            AccountMeta::new_readonly(ix.system_program, false),
        ];
        let mut data = vec![2u8];
        data.extend_from_slice(&ix.duel_id.to_le_bytes());
        data.extend_from_slice(&ix.stake.to_le_bytes());
        data.extend_from_slice(&ix.expiry.to_le_bytes());
        Instruction {
            program_id: ID,
            accounts,
            data,
        }
    }
}

// ---------------------------------------------------------------------------
// accept (discriminator = 3)
// ---------------------------------------------------------------------------

pub struct AcceptInstruction {
    pub opponent: Address,
    pub duel: Address,
    pub mint: Address,
    pub opponent_ta: Address,
    pub vault: Address,
    pub token_program: Address,
    pub challenger_key: Address,
    pub duel_id: u64,
}

impl From<AcceptInstruction> for Instruction {
    fn from(ix: AcceptInstruction) -> Instruction {
        let accounts = vec![
            AccountMeta::new(ix.opponent, true),
            AccountMeta::new(ix.duel, false),
            AccountMeta::new_readonly(ix.mint, false),
            AccountMeta::new(ix.opponent_ta, false),
            AccountMeta::new(ix.vault, false),
            AccountMeta::new_readonly(ix.token_program, false),
        ];
        let mut data = vec![3u8];
        data.extend_from_slice(ix.challenger_key.as_ref());
        data.extend_from_slice(&ix.duel_id.to_le_bytes());
        Instruction {
            program_id: ID,
            accounts,
            data,
        }
    }
}

// ---------------------------------------------------------------------------
// resolve (discriminator = 4)
// ---------------------------------------------------------------------------

pub struct ResolveInstruction {
    pub admin: Address,
    pub config: Address,
    pub duel: Address,
    pub treasury: Address,
    pub winner_account: Address,
    pub mint: Address,
    pub winner_ta: Address,
    pub treasury_ta: Address,
    pub vault: Address,
    pub rent: Address,
    pub token_program: Address,
    pub system_program: Address,
    pub challenger_key: Address,
    pub duel_id: u64,
    pub winner: u8,
}

impl From<ResolveInstruction> for Instruction {
    fn from(ix: ResolveInstruction) -> Instruction {
        let accounts = vec![
            AccountMeta::new(ix.admin, true),
            AccountMeta::new_readonly(ix.config, false),
            AccountMeta::new(ix.duel, false),
            AccountMeta::new_readonly(ix.treasury, false),
            AccountMeta::new_readonly(ix.winner_account, false),
            AccountMeta::new_readonly(ix.mint, false),
            AccountMeta::new(ix.winner_ta, false),
            AccountMeta::new(ix.treasury_ta, false),
            AccountMeta::new(ix.vault, false),
            AccountMeta::new_readonly(ix.rent, false),
            AccountMeta::new_readonly(ix.token_program, false),
            AccountMeta::new_readonly(ix.system_program, false),
        ];
        let mut data = vec![4u8];
        data.extend_from_slice(ix.challenger_key.as_ref());
        data.extend_from_slice(&ix.duel_id.to_le_bytes());
        data.push(ix.winner);
        Instruction {
            program_id: ID,
            accounts,
            data,
        }
    }
}

// ---------------------------------------------------------------------------
// cancel (discriminator = 5)
// ---------------------------------------------------------------------------

pub struct CancelInstruction {
    pub canceller: Address,
    pub duel: Address,
    pub mint: Address,
    pub challenger_ta: Address,
    pub opponent_ta: Address,
    pub vault: Address,
    pub rent: Address,
    pub token_program: Address,
    pub system_program: Address,
    pub challenger_key: Address,
    pub duel_id: u64,
}

impl From<CancelInstruction> for Instruction {
    fn from(ix: CancelInstruction) -> Instruction {
        let accounts = vec![
            AccountMeta::new(ix.canceller, true),
            AccountMeta::new(ix.duel, false),
            AccountMeta::new_readonly(ix.mint, false),
            AccountMeta::new(ix.challenger_ta, false),
            AccountMeta::new(ix.opponent_ta, false),
            AccountMeta::new(ix.vault, false),
            AccountMeta::new_readonly(ix.rent, false),
            AccountMeta::new_readonly(ix.token_program, false),
            AccountMeta::new_readonly(ix.system_program, false),
        ];
        let mut data = vec![5u8];
        data.extend_from_slice(ix.challenger_key.as_ref());
        data.extend_from_slice(&ix.duel_id.to_le_bytes());
        Instruction {
            program_id: ID,
            accounts,
            data,
        }
    }
}
