use {
    crate::{
        error::CinisError,
        events::ResolveEvent,
        state::{Config, Duel, STATUS_ACTIVE},
    },
    quasar_lang::prelude::*,
    quasar_spl::{validate_token_account, Mint, Token, TokenCpi},
};

#[derive(Accounts)]
#[instruction(challenger_key: Address, duel_id: u64)]
pub struct Resolve<'info> {
    #[account(mut)]
    pub admin: &'info mut Signer,
    #[account(
        has_one = admin,
        has_one = treasury,
        seeds = Config::seeds(),
        bump = config.bump
    )]
    pub config: &'info Account<Config>,
    #[account(
        mut,
        has_one = mint,
        constraint = duel.status == STATUS_ACTIVE @ CinisError::NotActive,
        close = admin,
        seeds = Duel::seeds(challenger_key, duel_id),
        bump = duel.bump
    )]
    pub duel: &'info mut Account<Duel>,
    /// CHECK: wallet whose address matches config.treasury via has_one
    pub treasury: &'info UncheckedAccount,
    /// CHECK: validated in `validate_winner`
    pub winner_account: &'info UncheckedAccount,
    pub mint: &'info Account<Mint>,
    #[account(mut)]
    pub winner_ta: &'info mut Account<Token>,
    #[account(mut, token::mint = mint, token::authority = treasury)]
    pub treasury_ta: &'info mut Account<Token>,
    #[account(mut, token::mint = mint, token::authority = duel)]
    pub vault: &'info mut Account<Token>,
    pub rent: &'info Sysvar<Rent>,
    pub token_program: &'info Program<Token>,
    pub system_program: &'info Program<System>,
}

impl<'info> Resolve<'info> {
    #[inline(always)]
    pub fn validate_winner(&self, winner: u8) -> Result<(), ProgramError> {
        // Duel is guaranteed ACTIVE by the account constraint, which means
        // accept() has run and opponent is non-default — no defensive check
        // needed here.
        let expected = match winner {
            0 => self.duel.challenger,
            1 => self.duel.opponent,
            _ => return Err(CinisError::InvalidWinner.into()),
        };

        if self.winner_account.address().as_ref() != expected.as_ref() {
            return Err(ProgramError::InvalidAccountData);
        }
        let winner_addr = self.winner_account.address();
        validate_token_account(
            self.winner_ta.to_account_view(),
            self.mint.address(),
            winner_addr,
            self.token_program.address(),
        )?;
        Ok(())
    }

    #[inline(always)]
    pub fn pay_fee(&mut self, bumps: &ResolveBumps) -> Result<(), ProgramError> {
        let stake = self.duel.stake.get() as u128;
        let bps = self.config.fee_bps.get() as u128;
        let fee = stake
            .checked_mul(2)
            .ok_or(ProgramError::ArithmeticOverflow)?
            .checked_mul(bps)
            .ok_or(ProgramError::ArithmeticOverflow)?
            .checked_div(10_000)
            .ok_or(ProgramError::ArithmeticOverflow)? as u64;

        if fee > 0 {
            let seeds = self.duel_seeds(bumps);
            self.token_program
                .transfer(self.vault, self.treasury_ta, self.duel, fee)
                .invoke_signed(&seeds)?;
        }
        Ok(())
    }

    #[inline(always)]
    pub fn pay_winner(&mut self, bumps: &ResolveBumps) -> Result<(), ProgramError> {
        let seeds = self.duel_seeds(bumps);
        let remaining = self.vault.amount();
        self.token_program
            .transfer(self.vault, self.winner_ta, self.duel, remaining)
            .invoke_signed(&seeds)
    }

    #[inline(always)]
    pub fn close_vault(&mut self, bumps: &ResolveBumps) -> Result<(), ProgramError> {
        let seeds = self.duel_seeds(bumps);
        self.token_program
            .close_account(self.vault, self.admin, self.duel)
            .invoke_signed(&seeds)
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
