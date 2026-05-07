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
// Status bytes (Duel.status)
// ---------------------------------------------------------------------------

/// Duel created, no opponent yet, awaiting an `accept`.
pub const STATUS_PENDING: u8 = 0;
/// Duel accepted, both stakes locked, awaiting a `resolve`.
pub const STATUS_ACTIVE: u8 = 1;

// ---------------------------------------------------------------------------
// Decode error
// ---------------------------------------------------------------------------

/// Returned when account bytes don't match the expected layout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeError {
    /// Account data length didn't match the struct's fixed length.
    InvalidLength { expected: usize, actual: usize },
    /// Discriminator byte didn't match the expected value.
    InvalidDiscriminator { expected: u8, actual: u8 },
}

impl core::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidLength { expected, actual } => write!(
                f,
                "account data length mismatch: expected {expected}, got {actual}"
            ),
            Self::InvalidDiscriminator { expected, actual } => write!(
                f,
                "account discriminator mismatch: expected {expected}, got {actual}"
            ),
        }
    }
}

impl std::error::Error for DecodeError {}

// ---------------------------------------------------------------------------
// Account decoders
// ---------------------------------------------------------------------------

/// Singleton program configuration. PDA at `[b"config"]`.
///
/// Layout: `disc(1) + admin(32) + treasury(32) + fee_bps(u16 LE) + bump(1) = 68 bytes`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub admin: Address,
    pub treasury: Address,
    pub fee_bps: u16,
    pub bump: u8,
}

impl Config {
    pub const DISCRIMINATOR: u8 = 1;
    pub const LEN: usize = 68;

    pub fn try_from_account_data(data: &[u8]) -> Result<Self, DecodeError> {
        if data.len() != Self::LEN {
            return Err(DecodeError::InvalidLength {
                expected: Self::LEN,
                actual: data.len(),
            });
        }
        if data[0] != Self::DISCRIMINATOR {
            return Err(DecodeError::InvalidDiscriminator {
                expected: Self::DISCRIMINATOR,
                actual: data[0],
            });
        }
        Ok(Self {
            admin: Address::new_from_array(data[1..33].try_into().unwrap()),
            treasury: Address::new_from_array(data[33..65].try_into().unwrap()),
            fee_bps: u16::from_le_bytes(data[65..67].try_into().unwrap()),
            bump: data[67],
        })
    }
}

/// Per-challenger tip-tracker. PDA at `[b"challenger", challenger]`.
///
/// Layout: `disc(1) + next_id(u64 LE) + bump(1) = 10 bytes`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Challenger {
    pub next_id: u64,
    pub bump: u8,
}

impl Challenger {
    pub const DISCRIMINATOR: u8 = 2;
    pub const LEN: usize = 10;

    pub fn try_from_account_data(data: &[u8]) -> Result<Self, DecodeError> {
        if data.len() != Self::LEN {
            return Err(DecodeError::InvalidLength {
                expected: Self::LEN,
                actual: data.len(),
            });
        }
        if data[0] != Self::DISCRIMINATOR {
            return Err(DecodeError::InvalidDiscriminator {
                expected: Self::DISCRIMINATOR,
                actual: data[0],
            });
        }
        Ok(Self {
            next_id: u64::from_le_bytes(data[1..9].try_into().unwrap()),
            bump: data[9],
        })
    }
}

/// Wager state between two parties. PDA at `[b"duel", challenger, duel_id.to_le_bytes()]`.
///
/// Layout: `disc(1) + challenger(32) + opponent(32) + mint(32) + stake(u64 LE)
///   + expiry(i64 LE) + duel_id(u64 LE) + status(1) + bump(1) = 123 bytes`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Duel {
    pub challenger: Address,
    pub opponent: Address,
    pub mint: Address,
    pub stake: u64,
    pub expiry: i64,
    pub duel_id: u64,
    pub status: u8,
    pub bump: u8,
}

impl Duel {
    pub const DISCRIMINATOR: u8 = 3;
    pub const LEN: usize = 123;

    pub fn try_from_account_data(data: &[u8]) -> Result<Self, DecodeError> {
        if data.len() != Self::LEN {
            return Err(DecodeError::InvalidLength {
                expected: Self::LEN,
                actual: data.len(),
            });
        }
        if data[0] != Self::DISCRIMINATOR {
            return Err(DecodeError::InvalidDiscriminator {
                expected: Self::DISCRIMINATOR,
                actual: data[0],
            });
        }
        Ok(Self {
            challenger: Address::new_from_array(data[1..33].try_into().unwrap()),
            opponent: Address::new_from_array(data[33..65].try_into().unwrap()),
            mint: Address::new_from_array(data[65..97].try_into().unwrap()),
            stake: u64::from_le_bytes(data[97..105].try_into().unwrap()),
            expiry: i64::from_le_bytes(data[105..113].try_into().unwrap()),
            duel_id: u64::from_le_bytes(data[113..121].try_into().unwrap()),
            status: data[121],
            bump: data[122],
        })
    }
}

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
