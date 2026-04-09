use {
    crate::{
        error::CinisError,
        events::ResolveEvent,
        state::{Duel, STATUS_ACTIVE},
    },
    quasar_lang::prelude::*,
    quasar_spl::{Mint, Token, TokenClose, TokenCpi},
};

#[derive(Accounts)]
pub struct Resolve<'info> {
    pub authority: &'info Signer,
    #[account(
        has_one = authority,
        has_one = challenger,
        has_one = fee_account,
        constraint = duel.status == STATUS_ACTIVE @ CinisError::NotActive,
        close = challenger,
        seeds = [b"duel", challenger],
        bump = duel.bump
    )]
    pub duel: &'info mut Account<Duel>,
    pub challenger: &'info mut UncheckedAccount,
    pub mint: &'info Account<Mint>,
    #[account(init_if_needed, payer = challenger, token::mint = mint, token::authority = challenger)]
    pub winner_ta: &'info mut Account<Token>,
    pub fee_account: &'info mut Account<Token>,
    pub vault: &'info mut Account<Token>,
    pub rent: &'info Sysvar<Rent>,
    pub token_program: &'info Program<Token>,
    pub system_program: &'info Program<System>,
}

impl<'info> Resolve<'info> {
    #[inline(always)]
    pub fn validate_winner(&self, winner: u8) -> Result<(), ProgramError> {
        if winner > 1 {
            return Err(CinisError::InvalidWinner.into());
        }
        Ok(())
    }

    #[inline(always)]
    pub fn pay_fee(&mut self, bumps: &ResolveBumps) -> Result<(), ProgramError> {
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
            let seeds = bumps.duel_seeds();
            self.token_program
                .transfer(self.vault, self.fee_account, self.duel, fee)
                .invoke_signed(&seeds)?;
        }
        Ok(())
    }

    #[inline(always)]
    pub fn pay_winner(&mut self, _winner: u8, bumps: &ResolveBumps) -> Result<(), ProgramError> {
        let seeds = bumps.duel_seeds();
        let remaining = self.vault.amount();

        self.token_program
            .transfer(self.vault, self.winner_ta, self.duel, remaining)
            .invoke_signed(&seeds)
    }

    #[inline(always)]
    pub fn close_vault(&mut self, bumps: &ResolveBumps) -> Result<(), ProgramError> {
        let seeds = bumps.duel_seeds();
        self.vault
            .close(self.token_program, self.challenger, self.duel)
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
