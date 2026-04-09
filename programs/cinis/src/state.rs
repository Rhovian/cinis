use quasar_lang::prelude::*;

/// Status byte for the duel account.
pub const STATUS_PENDING: u8 = 0;
pub const STATUS_ACTIVE: u8 = 1;

/// Duel account — stores the state of a wager between two parties.
///
/// Seeds: `[b"duel", challenger]`
///
/// NOTE: One active duel per challenger until blueshift-gg/quasar#115 is
/// resolved, then seeds become `[b"duel", challenger, &duel_id.to_le_bytes()]`.
///
/// Layout (173 bytes):
///   disc(1) + challenger(32) + opponent(32) + mint(32) + authority(32)
///   + fee_account(32) + stake(8) + expiry(8) + fee_bps(2) + status(1) + bump(1)
#[account(discriminator = 1)]
pub struct Duel {
    pub challenger: Address,
    pub opponent: Address,
    pub mint: Address,
    pub authority: Address,
    pub fee_account: Address,
    pub stake: u64,
    pub expiry: i64,
    pub fee_bps: u16,
    pub status: u8,
    pub bump: u8,
}
