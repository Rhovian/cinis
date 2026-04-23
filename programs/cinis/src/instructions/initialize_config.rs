use {
    crate::{
        error::CinisError,
        events::InitializeConfigEvent,
        state::{Config, ConfigInner},
    },
    quasar_lang::prelude::*,
};

#[derive(Accounts)]
pub struct InitializeConfig {
    #[account(mut)]
    pub admin: Signer,
    #[account(mut, init, payer = admin, seeds = Config::seeds(), bump)]
    pub config: Account<Config>,
    /// CHECK: recorded as treasury wallet; fee ATAs are derived from this
    pub treasury: UncheckedAccount,
    pub rent: Sysvar<Rent>,
    pub system_program: Program<System>,
}

impl InitializeConfig {
    #[inline(always)]
    pub fn init_config(
        &mut self,
        fee_bps: u16,
        bumps: &InitializeConfigBumps,
    ) -> Result<(), ProgramError> {
        if fee_bps > 10_000 {
            return Err(CinisError::FeeTooHigh.into());
        }
        self.config.set_inner(ConfigInner {
            admin: *self.admin.address(),
            treasury: *self.treasury.address(),
            fee_bps,
            bump: bumps.config,
        });
        Ok(())
    }

    #[inline(always)]
    pub fn emit_event(&self, fee_bps: u16) -> Result<(), ProgramError> {
        emit!(InitializeConfigEvent {
            config: *self.config.address(),
            admin: *self.admin.address(),
            treasury: *self.treasury.address(),
            fee_bps: fee_bps as u64,
        });
        Ok(())
    }
}
