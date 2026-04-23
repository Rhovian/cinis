use {
    crate::{
        error::CinisError,
        events::CreateEvent,
        state::{Challenger, ChallengerInner, Config, Duel, DuelInner, STATUS_PENDING},
    },
    quasar_lang::prelude::*,
    quasar_spl::{AssociatedTokenProgram, Mint, Token, TokenCpi},
};

#[derive(Accounts)]
#[instruction(duel_id: u64)]
pub struct Create {
    #[account(mut)]
    pub challenger: Signer,
    #[account(seeds = Config::seeds(), bump = config.bump)]
    pub config: Account<Config>,
    #[account(
        mut,
        init_if_needed,
        payer = challenger,
        seeds = Challenger::seeds(challenger),
        bump
    )]
    pub challenger_state: Account<Challenger>,
    #[account(
        mut,
        init,
        payer = challenger,
        seeds = Duel::seeds(challenger, duel_id),
        bump
    )]
    pub duel: Account<Duel>,
    pub mint: Account<Mint>,
    #[account(mut)]
    pub challenger_ta: Account<Token>,
    #[account(
        mut,
        init_if_needed,
        payer = challenger,
        associated_token::mint = mint,
        associated_token::authority = duel
    )]
    pub vault: Account<Token>,
    pub rent: Sysvar<Rent>,
    pub token_program: Program<Token>,
    pub associated_token_program: Program<AssociatedTokenProgram>,
    pub system_program: Program<System>,
}

impl Create {
    #[inline(always)]
    pub fn tick_tip(
        &mut self,
        duel_id: u64,
        bumps: &CreateBumps,
    ) -> Result<(), ProgramError> {
        let current = self.challenger_state.next_id.get();
        if current != duel_id {
            return Err(CinisError::InvalidDuelId.into());
        }
        self.challenger_state.set_inner(ChallengerInner {
            next_id: duel_id + 1,
            bump: bumps.challenger_state,
        });
        Ok(())
    }

    #[inline(always)]
    pub fn create_duel(
        &mut self,
        duel_id: u64,
        stake: u64,
        expiry: i64,
        bumps: &CreateBumps,
    ) -> Result<(), ProgramError> {
        self.duel.set_inner(DuelInner {
            challenger: *self.challenger.address(),
            opponent: Address::default(),
            mint: *self.mint.address(),
            stake,
            expiry,
            duel_id,
            status: STATUS_PENDING,
            bump: bumps.duel,
        });
        Ok(())
    }

    #[inline(always)]
    pub fn deposit_stake(&mut self, stake: u64) -> Result<(), ProgramError> {
        self.token_program
            .transfer(&self.challenger_ta, &self.vault, &self.challenger, stake)
            .invoke()
    }

    #[inline(always)]
    pub fn emit_event(
        &self,
        duel_id: u64,
        stake: u64,
        expiry: i64,
    ) -> Result<(), ProgramError> {
        emit!(CreateEvent {
            duel: *self.duel.address(),
            challenger: *self.challenger.address(),
            mint: *self.mint.address(),
            duel_id,
            stake,
            expiry,
        });
        Ok(())
    }
}
