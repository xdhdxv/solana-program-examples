use solana_program::program_error::ProgramError;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum SwapProgramError {
    // error 0
    #[error("")]
    InvalidSwapZeroAmount,
    // error 1
    #[error("")]
    InvalidSwapMatchingAssets,
}

impl From<SwapProgramError> for ProgramError {
    fn from(e: SwapProgramError) -> Self {
        ProgramError::Custom(e as u32)
    }
}