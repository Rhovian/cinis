use {
    crate::{error::CinisError, events::CreateEvent, state::Duel},
    quasar_lang::prelude::*,
    quasar_spl::{Mint, Token, TokenCpi},
};

#[derive(Accounts)]
pub struct Create<'info> {
    pub challenger: &'info mut Signer,
    #[account(init, payer = challenger, seeds = [b"duel", challenger], bump)]
    pub duel: &'info mut Account<Duel>,
    pub authority: &'info UncheckedAccount,
    pub mint: &'info Account<Mint>,
    pub challenger_ta: &'info mut Account<Token>,
    pub fee_account: &'info Account<Token>,
    #[account(init_if_needed, payer = challenger, token::mint = mint, token::authority = duel)]
    pub vault: &'info mut Account<Token>,
    pub rent: &'info Sysvar<Rent>,
    pub token_program: &'info Program<Token>,
    pub system_program: &'info Program<System>,
}

impl<'info> Create<'info> {
    #[inline(always)]
    pub fn create_duel(
        &mut self,
        stake: u64,
        fee_bps: u16,
        expiry: i64,
        bumps: &CreateBumps,
    ) -> Result<(), ProgramError> {
        if fee_bps > 10_000 {
            return Err(CinisError::FeeTooHigh.into());
        }
        self.duel.set_inner(
            *self.challenger.address(),
            Address::default(),
            *self.mint.address(),
            *self.authority.address(),
            *self.fee_account.address(),
            stake,
            expiry,
            fee_bps,
            crate::state::STATUS_PENDING,
            bumps.duel,
        );
        Ok(())
    }

    #[inline(always)]
    pub fn deposit_stake(&mut self, stake: u64) -> Result<(), ProgramError> {
        self.token_program
            .transfer(self.challenger_ta, self.vault, self.challenger, stake)
            .invoke()
    }

    #[inline(always)]
    pub fn emit_event(&self, stake: u64, fee_bps: u16, expiry: i64) -> Result<(), ProgramError> {
        emit!(CreateEvent {
            duel: *self.duel.address(),
            challenger: *self.challenger.address(),
            mint: *self.mint.address(),
            authority: *self.authority.address(),
            stake,
            expiry,
            fee_bps: fee_bps as u64,
        });
        Ok(())
    }
}
