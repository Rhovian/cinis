use quasar_lang::prelude::*;

#[event(discriminator = 0)]
pub struct CreateEvent {
    pub duel: Address,
    pub challenger: Address,
    pub mint: Address,
    pub authority: Address,
    pub stake: u64,
    pub expiry: i64,
    pub fee_bps: u64,
}

#[event(discriminator = 1)]
pub struct AcceptEvent {
    pub duel: Address,
    pub opponent: Address,
}

#[event(discriminator = 2)]
pub struct ResolveEvent {
    pub duel: Address,
    pub winner: u8,
}

#[event(discriminator = 3)]
pub struct CancelEvent {
    pub duel: Address,
}
