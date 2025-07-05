use solana_program::{
    entrypoint::ProgramResult,
    program_error::ProgramError,
    pubkey::Pubkey,
    account_info::{AccountInfo, next_account_info},
    sysvar::{Sysvar, rent::Rent},
    program::invoke_signed,
    program_pack::IsInitialized,
    msg,
    borsh1::try_from_slice_unchecked,
};

use borsh::BorshSerialize;

use crate::instruction::MovieInstruction;
use crate::state::MovieAccountState;
use crate::error::ReviewError;

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction = MovieInstruction::unpack(instruction_data)?;

    match instruction {
        MovieInstruction::AddMovieReview { title, rating, description } => {
            process_add_movie_review(program_id, accounts, title, rating, description)
        },
        MovieInstruction::UpdateMovieReview { title, rating, description } => {
            process_update_movie_review(program_id, accounts, title, rating, description)
        },
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

    let accounts_iter = &mut accounts.iter();
    
    let reviewer = next_account_info(accounts_iter)?;
    let movie_review_account = next_account_info(accounts_iter)?;
    let system_program = next_account_info(accounts_iter)?;

    if !reviewer.is_signer {
        return Err(ProgramError::MissingRequiredSignature)
    }

    let (pda, bump_seed) = Pubkey::find_program_address(
        &[reviewer.key.as_ref(), title.as_bytes().as_ref()], 
        program_id,
    );

    if *movie_review_account.key != pda {
        return Err(ProgramError::InvalidSeeds);
    }

    if rating < 1 || rating > 5 {
        return Err(ReviewError::InvalidRating.into());
    }

    let total_len = 1 + 1 + (4 + title.len()) + (4 + description.len());
    if total_len > 1000 {
        return Err(ReviewError::InvalidDataLength.into());
    }

    let movie_account_space = 1000;

    let rent = Rent::get()?;

    let movie_account_rent = rent.minimum_balance(movie_account_space);

    invoke_signed(
        &solana_system_interface::instruction::create_account(
            reviewer.key, 
            movie_review_account.key, 
            movie_account_rent, 
            movie_account_space as u64, 
            program_id,
        ), 
        &[
            reviewer.clone(), 
            movie_review_account.clone(), 
            system_program.clone(),
        ], 
        &[
            &[
                reviewer.key.as_ref(), 
                title.as_bytes().as_ref(),
                &[bump_seed],
            ],
        ]
    )?;

    msg!("Movie Review Account created: {}", movie_review_account.key);

    msg!("Unpacking movie review account");
    let mut movie_review_account_data = 
        try_from_slice_unchecked::<MovieAccountState>(&movie_review_account.data.borrow())?;   
    msg!("Borrowed account data");

    if movie_review_account_data.is_initialized() {
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    movie_review_account_data.title = title;
    movie_review_account_data.rating = rating;
    movie_review_account_data.description = description;
    movie_review_account_data.is_initialized = true;

    msg!("Serializing account");
    movie_review_account_data.serialize(&mut &mut movie_review_account.data.borrow_mut()[..])?;
    msg!("Movie review account serialized");

    Ok(())
}

pub fn process_update_movie_review(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    title: String,
    rating: u8,
    description: String
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();

    let reviewer = next_account_info(accounts_iter)?;
    let movie_review_account = next_account_info(accounts_iter)?;

    if !reviewer.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if movie_review_account.owner != program_id {
        return Err(ProgramError::InvalidAccountOwner);
    }

    let (pda, _bump_seed) = Pubkey::find_program_address(
        &[reviewer.key.as_ref(), title.as_bytes().as_ref()], 
        program_id,
    );

    if *movie_review_account.key != pda {
        return Err(ProgramError::InvalidSeeds);
    }

    msg!("Unpacking state account");
    let mut movie_review_account_data = 
        try_from_slice_unchecked::<MovieAccountState>(&movie_review_account.data.borrow())?;
    msg!("Borrowed account data");

    if !movie_review_account_data.is_initialized() {
        return Err(ProgramError::UninitializedAccount);
    }

    if rating < 1 || rating > 5 {
        return Err(ReviewError::InvalidRating.into());
    }

    let total_len = 1 + 1 + (4 + title.len()) + (4 + description.len());
    if total_len > 1000 {
        return Err(ReviewError::InvalidDataLength.into());
    }

    movie_review_account_data.rating = rating;
    movie_review_account_data.description = description;

    movie_review_account_data.serialize(&mut &mut movie_review_account.data.borrow_mut()[..])?;

    Ok(())
}