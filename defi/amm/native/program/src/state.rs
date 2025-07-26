use solana_program::pubkey::Pubkey;

use borsh::{BorshSerialize, BorshDeserialize};

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct LiquidityPool {
    pub mint_a: Pubkey,
    pub mint_b: Pubkey,
    pub reserve_a: u64,
    pub reserve_b: u64,
    pub fee_bps: u16,
    pub bump: u8,
}

impl LiquidityPool {
    pub const SPACE: usize = 
        32       // mint_a pubkey
        + 32     // mint_b pubkey
        + 8      // reserve_a 
        + 8      // reserve_b 
        + 2      // fee_bps
        + 1;     // bump
}