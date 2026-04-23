use {
    crate::{
        error::CinisError,
        events::CancelEvent,
        state::{Duel, STATUS_ACTIVE, STATUS_PENDING},
    },
    quasar_lang::{
        prelude::*,
        sysvars::{clock::Clock, Sysvar as _},
    },
    quasar_spl::{validate_token_account, Mint, Token, TokenCpi},
};

#[derive(Accounts)]
#[instruction(challenger_key: Address, duel_id: u64)]
pub struct Cancel {
    #[account(mut)]
    pub canceller: Signer,
    #[account(
        mut,
        has_one = mint,
        close = canceller,
        seeds = Duel::seeds(challenger_key, duel_id),
        bump = duel.bump
    )]
    pub duel: Account<Duel>,
    pub mint: Account<Mint>,
    #[account(mut)]
    pub challenger_ta: Account<Token>,
    #[account(mut)]
    pub opponent_ta: Account<Token>,
    #[account(mut, token::mint = mint, token::authority = duel)]
    pub vault: Account<Token>,
    pub rent: Sysvar<Rent>,
    pub token_program: Program<Token>,
    pub system_program: Program<System>,
}

impl Cancel {
    #[inline(always)]
    pub fn validate_cancel(&self) -> Result<(), ProgramError> {
        let canceller = self.canceller.address();
        let challenger = &self.duel.challenger;

        match self.duel.status {
            STATUS_PENDING => {
                // Challenger can always cancel; anyone else only after expiry.
                if canceller.as_ref() != challenger.as_ref() {
                    let now = Clock::get()?.unix_timestamp.get();
                    if now <= self.duel.expiry.get() {
                        return Err(CinisError::Unauthorized.into());
                    }
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

        validate_token_account(
            self.challenger_ta.to_account_view(),
            self.mint.address(),
            challenger,
            self.token_program.address(),
        )?;

        if self.duel.status == STATUS_ACTIVE {
            validate_token_account(
                self.opponent_ta.to_account_view(),
                self.mint.address(),
                &self.duel.opponent,
                self.token_program.address(),
            )?;
        }
        Ok(())
    }

    #[inline(always)]
    pub fn withdraw_and_close(&mut self, bumps: &CancelBumps) -> Result<(), ProgramError> {
        let challenger = self.duel.challenger;
        let duel_id_bytes = self.duel.duel_id.get().to_le_bytes();
        let bump = [bumps.duel];
        let seeds = [
            quasar_lang::cpi::Seed::from(b"duel" as &[u8]),
            quasar_lang::cpi::Seed::from(challenger.as_ref()),
            quasar_lang::cpi::Seed::from(&duel_id_bytes as &[u8]),
            quasar_lang::cpi::Seed::from(&bump as &[u8]),
        ];
        let stake = self.duel.stake.get();

        if self.duel.status == STATUS_ACTIVE {
            self.token_program
                .transfer(&self.vault, &self.opponent_ta, &self.duel, stake)
                .invoke_signed(&seeds)?;
        }

        let remaining = self.vault.amount();
        self.token_program
            .transfer(&self.vault, &self.challenger_ta, &self.duel, remaining)
            .invoke_signed(&seeds)?;

        self.token_program
            .close_account(&self.vault, &self.canceller, &self.duel)
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
