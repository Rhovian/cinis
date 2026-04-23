use quasar_lang::prelude::*;

#[event(discriminator = 0)]
pub struct InitializeConfigEvent {
    pub config: Address,
    pub admin: Address,
    pub treasury: Address,
    pub fee_bps: u64,
}

#[event(discriminator = 1)]
pub struct UpdateConfigEvent {
    pub config: Address,
    pub admin: Address,
    pub treasury: Address,
    pub fee_bps: u64,
}

#[event(discriminator = 2)]
pub struct CreateEvent {
    pub duel: Address,
    pub challenger: Address,
    pub mint: Address,
    pub duel_id: u64,
    pub stake: u64,
    pub expiry: i64,
}

#[event(discriminator = 3)]
pub struct AcceptEvent {
    pub duel: Address,
    pub opponent: Address,
}

#[event(discriminator = 4)]
pub struct ResolveEvent {
    pub duel: Address,
    pub winner: u8,
}

#[event(discriminator = 5)]
pub struct CancelEvent {
    pub duel: Address,
}
