use {
    crate::{
        error::CinisError,
        events::ResolveEvent,
        state::{Duel, STATUS_ACTIVE},
    },
    quasar_lang::prelude::*,
    quasar_spl::{validate_token_account, Mint, Token, TokenCpi},
};

#[derive(Accounts)]
pub struct Resolve<'info> {
    pub authority: &'info mut Signer,
    #[account(
        has_one = authority,
        has_one = mint,
        has_one = fee_account,
        constraint = duel.status == STATUS_ACTIVE @ CinisError::NotActive
    )]
    pub duel: &'info mut Account<Duel>,
    /// CHECK: validated in `validate_winner`
    pub winner_account: &'info UncheckedAccount,
    pub mint: &'info Account<Mint>,
    pub winner_ta: &'info mut Account<Token>,
    pub fee_account: &'info mut Account<Token>,
    #[account(token::mint = mint, token::authority = duel)]
    pub vault: &'info mut Account<Token>,
    pub rent: &'info Sysvar<Rent>,
    pub token_program: &'info Program<Token>,
    pub system_program: &'info Program<System>,
}

impl<'info> Resolve<'info> {
    #[inline(always)]
    pub fn validate_winner(&self, winner: u8) -> Result<(), ProgramError> {
        let (expected_duel, expected_bump) =
            Address::find_program_address(&[b"duel", self.duel.challenger.as_ref()], &crate::ID);
        if self.duel.address().as_ref() != expected_duel.as_ref() || self.duel.bump != expected_bump
        {
            return Err(ProgramError::InvalidAccountData);
        }

        let expected = match winner {
            0 => self.duel.challenger,
            1 => {
                if self.duel.opponent.as_ref() == Address::default().as_ref() {
                    return Err(ProgramError::InvalidAccountData);
                }
                self.duel.opponent
            }
            _ => return Err(CinisError::InvalidWinner.into()),
        };

        if self.winner_account.address().as_ref() != expected.as_ref() {
            return Err(ProgramError::InvalidAccountData);
        }
        let winner = self.winner_account.address();
        validate_token_account(
            self.winner_ta.to_account_view(),
            self.mint.address(),
            winner,
            self.token_program.address(),
        )?;

        Ok(())
    }

    #[inline(always)]
    pub fn pay_fee(&mut self) -> Result<(), ProgramError> {
        let stake = self.duel.stake.get() as u128;
        let bps = self.duel.fee_bps.get() as u128;
        let fee = stake
            .checked_mul(2)
            .unwrap()
            .checked_mul(bps)
            .unwrap()
            .checked_div(10_000)
            .unwrap() as u64;

        if fee > 0 {
            let bump = [self.duel.bump];
            let seeds = [
                quasar_lang::cpi::Seed::from(<Duel as quasar_lang::traits::HasSeeds>::SEED_PREFIX),
                quasar_lang::cpi::Seed::from(self.duel.challenger.as_ref()),
                quasar_lang::cpi::Seed::from(&bump),
            ];
            self.token_program
                .transfer(self.vault, self.fee_account, self.duel, fee)
                .invoke_signed(&seeds)?;
        }
        Ok(())
    }

    #[inline(always)]
    pub fn pay_winner(&mut self) -> Result<(), ProgramError> {
        let bump = [self.duel.bump];
        let seeds = [
            quasar_lang::cpi::Seed::from(<Duel as quasar_lang::traits::HasSeeds>::SEED_PREFIX),
            quasar_lang::cpi::Seed::from(self.duel.challenger.as_ref()),
            quasar_lang::cpi::Seed::from(&bump),
        ];
        let remaining = self.vault.amount();

        self.token_program
            .transfer(self.vault, self.winner_ta, self.duel, remaining)
            .invoke_signed(&seeds)
    }

    #[inline(always)]
    pub fn close_vault(&mut self) -> Result<(), ProgramError> {
        let bump = [self.duel.bump];
        let seeds = [
            quasar_lang::cpi::Seed::from(<Duel as quasar_lang::traits::HasSeeds>::SEED_PREFIX),
            quasar_lang::cpi::Seed::from(self.duel.challenger.as_ref()),
            quasar_lang::cpi::Seed::from(&bump),
        ];
        self.token_program
            .close_account(self.vault, self.authority, self.duel)
            .invoke_signed(&seeds)
    }

    #[inline(always)]
    pub fn close_duel(&mut self) -> Result<(), ProgramError> {
        self.duel.close(self.authority.to_account_view())
    }

    #[inline(always)]
    pub fn emit_event(&self, winner: u8) -> Result<(), ProgramError> {
        emit!(ResolveEvent {
            duel: *self.duel.address(),
            winner,
        });
        Ok(())
    }
}
