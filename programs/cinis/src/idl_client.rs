use {
    alloc::vec,
    solana_address::Address,
    solana_instruction::{AccountMeta, Instruction},
};

pub struct CreateInstruction {
    pub challenger: Address,
    pub duel: Address,
    pub authority: Address,
    pub mint: Address,
    pub challenger_ta: Address,
    pub fee_account: Address,
    pub vault: Address,
    pub rent: Address,
    pub token_program: Address,
    pub system_program: Address,
    pub stake: u64,
    pub fee_bps: u16,
    pub expiry: i64,
}

impl From<CreateInstruction> for Instruction {
    fn from(ix: CreateInstruction) -> Instruction {
        let accounts = vec![
            AccountMeta::new(ix.challenger, true),
            AccountMeta::new(ix.duel, false),
            AccountMeta::new_readonly(ix.authority, false),
            AccountMeta::new_readonly(ix.mint, false),
            AccountMeta::new(ix.challenger_ta, false),
            AccountMeta::new_readonly(ix.fee_account, false),
            AccountMeta::new(ix.vault, false),
            AccountMeta::new_readonly(ix.rent, false),
            AccountMeta::new_readonly(ix.token_program, false),
            AccountMeta::new_readonly(ix.system_program, false),
        ];
        let mut data = vec![0u8]; // discriminator 0
        data.extend_from_slice(&ix.stake.to_le_bytes());
        data.extend_from_slice(&ix.fee_bps.to_le_bytes());
        data.extend_from_slice(&ix.expiry.to_le_bytes());
        Instruction {
            program_id: crate::ID,
            accounts,
            data,
        }
    }
}

pub struct AcceptInstruction {
    pub opponent: Address,
    pub duel: Address,
    pub challenger: Address,
    pub opponent_ta: Address,
    pub vault: Address,
    pub token_program: Address,
}

impl From<AcceptInstruction> for Instruction {
    fn from(ix: AcceptInstruction) -> Instruction {
        let accounts = vec![
            AccountMeta::new(ix.opponent, true),
            AccountMeta::new(ix.duel, false),
            AccountMeta::new_readonly(ix.challenger, false),
            AccountMeta::new(ix.opponent_ta, false),
            AccountMeta::new(ix.vault, false),
            AccountMeta::new_readonly(ix.token_program, false),
        ];
        let data = vec![1u8]; // discriminator 1
        Instruction {
            program_id: crate::ID,
            accounts,
            data,
        }
    }
}

pub struct ResolveInstruction {
    pub authority: Address,
    pub duel: Address,
    pub winner_account: Address,
    pub mint: Address,
    pub winner_ta: Address,
    pub fee_account: Address,
    pub vault: Address,
    pub rent: Address,
    pub token_program: Address,
    pub system_program: Address,
    pub winner: u8,
}

impl From<ResolveInstruction> for Instruction {
    fn from(ix: ResolveInstruction) -> Instruction {
        let accounts = vec![
            AccountMeta::new(ix.authority, true),
            AccountMeta::new(ix.duel, false),
            AccountMeta::new_readonly(ix.winner_account, false),
            AccountMeta::new_readonly(ix.mint, false),
            AccountMeta::new(ix.winner_ta, false),
            AccountMeta::new(ix.fee_account, false),
            AccountMeta::new(ix.vault, false),
            AccountMeta::new_readonly(ix.rent, false),
            AccountMeta::new_readonly(ix.token_program, false),
            AccountMeta::new_readonly(ix.system_program, false),
        ];
        let data = vec![2u8, ix.winner]; // discriminator 2 + winner byte
        Instruction {
            program_id: crate::ID,
            accounts,
            data,
        }
    }
}

pub struct CancelInstruction {
    pub canceller: Address,
    pub duel: Address,
    pub mint: Address,
    pub challenger_ta: Address,
    pub opponent_ta: Address,
    pub vault: Address,
    pub rent: Address,
    pub token_program: Address,
    pub system_program: Address,
}

impl From<CancelInstruction> for Instruction {
    fn from(ix: CancelInstruction) -> Instruction {
        let accounts = vec![
            AccountMeta::new(ix.canceller, true),
            AccountMeta::new(ix.duel, false),
            AccountMeta::new_readonly(ix.mint, false),
            AccountMeta::new(ix.challenger_ta, false),
            AccountMeta::new(ix.opponent_ta, false),
            AccountMeta::new(ix.vault, false),
            AccountMeta::new_readonly(ix.rent, false),
            AccountMeta::new_readonly(ix.token_program, false),
            AccountMeta::new_readonly(ix.system_program, false),
        ];
        let data = vec![3u8]; // discriminator 3
        Instruction {
            program_id: crate::ID,
            accounts,
            data,
        }
    }
}
