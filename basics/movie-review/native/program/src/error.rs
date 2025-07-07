use solana_program::program_error::ProgramError;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ReviewError {
    // Error 0
    #[error("Input data exceeds max length")]
    InvalidDataLength,
    // Error 1
    #[error("Rating less than 1 or greater than 5")]
    InvalidRating,
    // Error 2
    #[error("Accounts do not match")]
    IncorrectAccountError
}

impl From<ReviewError> for ProgramError {
    fn from(e: ReviewError) -> Self {
        ProgramError::Custom(e as u32)
    }
}