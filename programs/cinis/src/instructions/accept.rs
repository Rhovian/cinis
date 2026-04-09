use {
    crate::{
        error::CinisError,
        events::AcceptEvent,
        state::{Duel, STATUS_PENDING},
    },
    quasar_lang::prelude::*,
    quasar_spl::{Token, TokenCpi},
};

#[derive(Accounts)]
pub struct Accept<'info> {
    pub opponent: &'info mut Signer,
    #[account(
        has_one = challenger,
        constraint = duel.status == STATUS_PENDING @ CinisError::NotPending,
        seeds = [b"duel", challenger],
        bump = duel.bump
    )]
    pub duel: &'info mut Account<Duel>,
    pub challenger: &'info UncheckedAccount,
    pub opponent_ta: &'info mut Account<Token>,
    pub vault: &'info mut Account<Token>,
    pub token_program: &'info Program<Token>,
}

impl<'info> Accept<'info> {
    #[inline(always)]
    pub fn accept_duel(&mut self) -> Result<(), ProgramError> {
        self.duel.opponent = *self.opponent.address();
        self.duel.status = crate::state::STATUS_ACTIVE;
        Ok(())
    }

    #[inline(always)]
    pub fn deposit_stake(&mut self) -> Result<(), ProgramError> {
        self.token_program
            .transfer(self.opponent_ta, self.vault, self.opponent, self.duel.stake)
            .invoke()
    }

    #[inline(always)]
    pub fn emit_event(&self) -> Result<(), ProgramError> {
        emit!(AcceptEvent {
            duel: *self.duel.address(),
            opponent: *self.opponent.address(),
        });
        Ok(())
    }
}
