use {
    crate::{
        error::CinisError,
        events::CancelEvent,
        state::{Duel, STATUS_ACTIVE, STATUS_PENDING},
    },
    quasar_lang::prelude::*,
    quasar_spl::{Mint, Token, TokenClose, TokenCpi},
};

#[derive(Accounts)]
pub struct Cancel<'info> {
    pub canceller: &'info mut Signer,
    #[account(
        has_one = challenger,
        close = challenger,
        seeds = [b"duel", challenger],
        bump = duel.bump
    )]
    pub duel: &'info mut Account<Duel>,
    pub challenger: &'info mut UncheckedAccount,
    pub mint: &'info Account<Mint>,
    #[account(init_if_needed, payer = canceller, token::mint = mint, token::authority = challenger)]
    pub challenger_ta: &'info mut Account<Token>,
    pub opponent_ta: &'info mut Account<Token>,
    pub vault: &'info mut Account<Token>,
    pub rent: &'info Sysvar<Rent>,
    pub token_program: &'info Program<Token>,
    pub system_program: &'info Program<System>,
}

impl<'info> Cancel<'info> {
    #[inline(always)]
    pub fn validate_cancel(&self) -> Result<(), ProgramError> {
        let canceller = self.canceller.address();
        let challenger = &self.duel.challenger;

        match self.duel.status {
            STATUS_PENDING => {
                if canceller.as_ref() != challenger.as_ref() {
                    return Err(CinisError::Unauthorized.into());
                }
            }
            STATUS_ACTIVE => {
                let opponent = &self.duel.opponent;
                let is_challenger = canceller.as_ref() == challenger.as_ref();
                let is_opponent = canceller.as_ref() == opponent.as_ref();
                if !is_challenger && !is_opponent {
                    return Err(CinisError::Unauthorized.into());
                }
            }
            _ => return Err(ProgramError::InvalidAccountData),
        }
        Ok(())
    }

    #[inline(always)]
    pub fn withdraw_and_close(&mut self, bumps: &CancelBumps) -> Result<(), ProgramError> {
        let seeds = bumps.duel_seeds();
        let stake = self.duel.stake.get();

        if self.duel.status == STATUS_ACTIVE {
            // Return opponent's stake
            self.token_program
                .transfer(self.vault, self.opponent_ta, self.duel, stake)
                .invoke_signed(&seeds)?;
        }

        // Return challenger's stake (remainder of vault)
        let remaining = self.vault.amount();
        self.token_program
            .transfer(self.vault, self.challenger_ta, self.duel, remaining)
            .invoke_signed(&seeds)?;

        self.vault
            .close(self.token_program, self.challenger, self.duel)
            .invoke_signed(&seeds)
    }

    #[inline(always)]
    pub fn emit_event(&self) -> Result<(), ProgramError> {
        emit!(CancelEvent {
            duel: *self.duel.address(),
        });
        Ok(())
    }
}
