use solana_program::pubkey::Pubkey;

use borsh::{BorshSerialize, BorshDeserialize};

#[derive(BorshSerialize, BorshDeserialize)]
pub struct LiquidityPool {
    pub assets: Vec<Pubkey>,
    pub bump: u8,
}

impl LiquidityPool {
    pub const SEED_PREFIX: &'static str = "liquidity_pool";

    pub const SPACE: usize = 
        4    // empty vector
        + 1; // 1 byte bump
}