pub mod instruction;

use solana_program::{
    entrypoint,
    entrypoint::ProgramResult,
    pubkey::Pubkey,
    account_info::AccountInfo,
    msg,
};

use instruction::MovieInstruction;

entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction = MovieInstruction::unpack(instruction_data)?;

    match instruction {
        MovieInstruction::AddMovieReview { title, rating, description } => {
            process_add_movie_review(program_id, accounts, title, rating, description)
        }
    }
}

pub fn process_add_movie_review(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    title: String,
    rating: u8,
    description: String,
) -> ProgramResult {
    msg!("Adding movie review...");
    msg!("Title: {}", title);
    msg!("Rating: {}", rating);
    msg!("Description: {}", description);

    Ok(())
}