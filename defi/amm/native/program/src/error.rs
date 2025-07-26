use thiserror::Error;

use solana_program::program_error::ProgramError;

#[derive(Error, Debug)]
pub enum AmmError {
    #[error("Token mints must be different")]
    IdenticalMints,
    #[error("Pool address does not match PDA derived from token mints")]
    PoolAddressMismatch,
    #[error("Vault address does not match ATA derived from mint and pool address")]
    VaultAddressMismatch,
    #[error("Mint address does not match pool data")]
    MintAddressMismatch,
    #[error("LP mint address does not match PDA derived from the pool")]
    LpMintAddressMismatch,
    #[error("Funding amount must be greater than zero")]
    ZeroLiquidityAmount,
    #[error("Fee must not exceed 10000 basis points (100%)")]
    FeeTooHigh,
    #[error("Swap amount must be greater than zero")]
    ZeroSwapAmount,
    #[error("Slippage tolerance exceeded: output amount is below the minimum specified")]
    SlippageExceed,
}

impl From<AmmError> for ProgramError {
    fn from(error: AmmError) -> Self {
        ProgramError::Custom(error as u32)
    }
}