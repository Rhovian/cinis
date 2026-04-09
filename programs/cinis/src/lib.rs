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
    pub fn create(
        ctx: Ctx<Create>,
        stake: u64,
        fee_bps: u16,
        expiry: i64,
    ) -> Result<(), ProgramError> {
        ctx.accounts
            .create_duel(stake, fee_bps, expiry, &ctx.bumps)?;
        ctx.accounts.deposit_stake(stake)?;
        ctx.accounts.emit_event(stake, fee_bps, expiry)
    }

    #[instruction(discriminator = 1)]
    pub fn accept(ctx: Ctx<Accept>) -> Result<(), ProgramError> {
        ctx.accounts.accept_duel()?;
        ctx.accounts.deposit_stake()?;
        ctx.accounts.emit_event()
    }

    #[instruction(discriminator = 2)]
    pub fn resolve(ctx: Ctx<Resolve>, winner: u8) -> Result<(), ProgramError> {
        ctx.accounts.validate_winner(winner)?;
        ctx.accounts.pay_fee()?;
        ctx.accounts.pay_winner()?;
        ctx.accounts.close_vault()?;
        ctx.accounts.close_duel()?;
        ctx.accounts.emit_event(winner)
    }

    #[instruction(discriminator = 3)]
    pub fn cancel(ctx: Ctx<Cancel>) -> Result<(), ProgramError> {
        ctx.accounts.validate_cancel()?;
        ctx.accounts.prepare_refund_accounts()?;
        ctx.accounts.withdraw_and_close()?;
        ctx.accounts.close_duel()?;
        ctx.accounts.emit_event()
    }
}
