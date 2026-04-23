use {
    crate::{
        error::CinisError,
        events::AcceptEvent,
        state::{Duel, STATUS_ACTIVE, STATUS_PENDING},
    },
    quasar_lang::{
        prelude::*,
        sysvars::{clock::Clock, Sysvar as _},
    },
    quasar_spl::{Mint, Token, TokenCpi},
};

#[derive(Accounts)]
#[instruction(challenger_key: Address, duel_id: u64)]
pub struct Accept {
    #[account(mut)]
    pub opponent: Signer,
    #[account(
        mut,
        has_one = mint,
        constraint = duel.status == STATUS_PENDING @ CinisError::NotPending,
        seeds = Duel::seeds(challenger_key, duel_id),
        bump = duel.bump
    )]
    pub duel: Account<Duel>,
    pub mint: Account<Mint>,
    #[account(mut)]
    pub opponent_ta: Account<Token>,
    #[account(mut, token::mint = mint, token::authority = duel)]
    pub vault: Account<Token>,
    pub token_program: Program<Token>,
}

impl Accept {
    #[inline(always)]
    pub fn accept_duel(&mut self) -> Result<(), ProgramError> {
        let now = Clock::get()?.unix_timestamp.get();
        if now > self.duel.expiry.get() {
            return Err(CinisError::Expired.into());
        }
        self.duel.opponent = *self.opponent.address();
        self.duel.status = STATUS_ACTIVE;
        Ok(())
    }

    #[inline(always)]
    pub fn deposit_stake(&mut self) -> Result<(), ProgramError> {
        let stake = self.duel.stake.get();
        self.token_program
            .transfer(&self.opponent_ta, &self.vault, &self.opponent, stake)
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
