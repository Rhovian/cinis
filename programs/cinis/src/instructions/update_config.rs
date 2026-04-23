use {
    crate::{
        error::CinisError,
        events::UpdateConfigEvent,
        state::{Config, ConfigInner},
    },
    quasar_lang::prelude::*,
};

#[derive(Accounts)]
pub struct UpdateConfig<'info> {
    pub admin: &'info Signer,
    #[account(mut, has_one = admin, seeds = Config::seeds(), bump = config.bump)]
    pub config: &'info mut Account<Config>,
    /// CHECK: recorded as new treasury wallet
    pub new_treasury: &'info UncheckedAccount,
}

impl<'info> UpdateConfig<'info> {
    #[inline(always)]
    pub fn update(&mut self, fee_bps: u16) -> Result<(), ProgramError> {
        if fee_bps > 10_000 {
            return Err(CinisError::FeeTooHigh.into());
        }
        let admin = self.config.admin;
        let bump = self.config.bump;
        self.config.set_inner(ConfigInner {
            admin,
            treasury: *self.new_treasury.address(),
            fee_bps,
            bump,
        });
        Ok(())
    }

    #[inline(always)]
    pub fn emit_event(&self, fee_bps: u16) -> Result<(), ProgramError> {
        emit!(UpdateConfigEvent {
            config: *self.config.address(),
            admin: *self.admin.address(),
            treasury: *self.new_treasury.address(),
            fee_bps: fee_bps as u64,
        });
        Ok(())
    }
}
