use quasar_lang::prelude::*;

/// Status byte for the duel account.
pub const STATUS_PENDING: u8 = 0;
pub const STATUS_ACTIVE: u8 = 1;

/// Global program configuration — singleton PDA.
///
/// Seeds: `[b"config"]`
#[account(discriminator = 1, set_inner)]
#[seeds(b"config")]
pub struct Config {
    pub admin: Address,
    pub treasury: Address,
    pub fee_bps: u16,
    pub bump: u8,
}

/// Per-challenger tip tracker.
///
/// Seeds: `[b"challenger", challenger]`
#[account(discriminator = 2, set_inner)]
#[seeds(b"challenger", challenger: Address)]
pub struct Challenger {
    pub next_id: u64,
    pub bump: u8,
}

/// Duel account — wager state between two parties.
///
/// Seeds: `[b"duel", challenger, duel_id.to_le_bytes()]`
#[account(discriminator = 3, set_inner)]
#[seeds(b"duel", challenger: Address, duel_id: u64)]
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
