use {
    crate::{
        error::CinisError,
        events::CancelEvent,
        state::{Duel, STATUS_ACTIVE, STATUS_PENDING},
    },
    quasar_lang::prelude::*,
    quasar_spl::{validate_token_account, Mint, Token, TokenCpi},
};

#[derive(Accounts)]
pub struct Cancel<'info> {
    pub canceller: &'info mut Signer,
    #[account(
        has_one = mint,
    )]
    pub duel: &'info mut Account<Duel>,
    pub mint: &'info Account<Mint>,
    pub challenger_ta: &'info mut Account<Token>,
    pub opponent_ta: &'info mut Account<Token>,
    #[account(token::mint = mint, token::authority = duel)]
    pub vault: &'info mut Account<Token>,
    pub rent: &'info Sysvar<Rent>,
    pub token_program: &'info Program<Token>,
    pub system_program: &'info Program<System>,
}

impl<'info> Cancel<'info> {
    #[inline(always)]
    pub fn validate_cancel(&self) -> Result<(), ProgramError> {
        let (expected_duel, expected_bump) =
            Address::find_program_address(&[b"duel", self.duel.challenger.as_ref()], &crate::ID);
        if self.duel.address().as_ref() != expected_duel.as_ref() || self.duel.bump != expected_bump
        {
            return Err(ProgramError::InvalidAccountData);
        }

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
    pub fn prepare_refund_accounts(&mut self) -> Result<(), ProgramError> {
        let challenger = self.duel.challenger;
        validate_token_account(
            self.challenger_ta.to_account_view(),
            self.mint.address(),
            &challenger,
            self.token_program.address(),
        )?;

        if self.duel.status == STATUS_ACTIVE {
            if self.duel.opponent.as_ref() == Address::default().as_ref() {
                return Err(ProgramError::InvalidAccountData);
            }
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
    pub fn withdraw_and_close(&mut self) -> Result<(), ProgramError> {
        let bump = [self.duel.bump];
        let seeds = [
            quasar_lang::cpi::Seed::from(<Duel as quasar_lang::traits::HasSeeds>::SEED_PREFIX),
            quasar_lang::cpi::Seed::from(self.duel.challenger.as_ref()),
            quasar_lang::cpi::Seed::from(&bump),
        ];
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

        self.token_program
            .close_account(self.vault, self.canceller, self.duel)
            .invoke_signed(&seeds)
    }

    #[inline(always)]
    pub fn close_duel(&mut self) -> Result<(), ProgramError> {
        self.duel.close(self.canceller.to_account_view())
    }

    #[inline(always)]
    pub fn emit_event(&self) -> Result<(), ProgramError> {
        emit!(CancelEvent {
            duel: *self.duel.address(),
        });
        Ok(())
    }
}
