#![no_std]

use quasar_lang::prelude::*;

mod error;
mod instructions;
use instructions::*;
mod events;
#[cfg(test)]
mod idl_client;
mod state;
#[cfg(test)]
mod tests;

declare_id!("11111111111111111111111111111112");

#[program]
mod cinis {
    use super::*;

    #[instruction(discriminator = 0)]
    pub fn initialize_config(
        ctx: Ctx<InitializeConfig>,
        fee_bps: u16,
    ) -> Result<(), ProgramError> {
        ctx.accounts.init_config(fee_bps, &ctx.bumps)?;
        ctx.accounts.emit_event(fee_bps)
    }

    #[instruction(discriminator = 1)]
    pub fn update_config(ctx: Ctx<UpdateConfig>, fee_bps: u16) -> Result<(), ProgramError> {
        ctx.accounts.update(fee_bps)?;
        ctx.accounts.emit_event(fee_bps)
    }

    #[instruction(discriminator = 2)]
    pub fn create(
        ctx: Ctx<Create>,
        duel_id: u64,
        stake: u64,
        expiry: i64,
    ) -> Result<(), ProgramError> {
        ctx.accounts.tick_tip(duel_id, &ctx.bumps)?;
        ctx.accounts.create_duel(duel_id, stake, expiry, &ctx.bumps)?;
        ctx.accounts.deposit_stake(stake)?;
        ctx.accounts.emit_event(duel_id, stake, expiry)
    }

    #[instruction(discriminator = 3)]
    pub fn accept(
        ctx: Ctx<Accept>,
        _challenger_key: Address,
        _duel_id: u64,
    ) -> Result<(), ProgramError> {
        ctx.accounts.accept_duel()?;
        ctx.accounts.deposit_stake()?;
        ctx.accounts.emit_event()
    }

    #[instruction(discriminator = 4)]
    pub fn resolve(
        ctx: Ctx<Resolve>,
        _challenger_key: Address,
        _duel_id: u64,
        winner: u8,
    ) -> Result<(), ProgramError> {
        ctx.accounts.validate_winner(winner)?;
        ctx.accounts.pay_fee(&ctx.bumps)?;
        ctx.accounts.pay_winner(&ctx.bumps)?;
        ctx.accounts.close_vault(&ctx.bumps)?;
        ctx.accounts.emit_event(winner)
    }

    #[instruction(discriminator = 5)]
    pub fn cancel(
        ctx: Ctx<Cancel>,
        _challenger_key: Address,
        _duel_id: u64,
    ) -> Result<(), ProgramError> {
        ctx.accounts.validate_cancel()?;
        ctx.accounts.withdraw_and_close(&ctx.bumps)?;
        ctx.accounts.emit_event()
    }
}
