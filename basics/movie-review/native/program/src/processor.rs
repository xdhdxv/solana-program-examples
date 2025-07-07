use solana_program::{
    entrypoint::ProgramResult,
    program_error::ProgramError,
    pubkey::Pubkey,
    account_info::{AccountInfo, next_account_info},
    sysvar::{Sysvar, rent::Rent},
    program::invoke_signed,
    program_pack::IsInitialized,
    borsh1::try_from_slice_unchecked,
    native_token::LAMPORTS_PER_SOL,
    program_pack::Pack,
};
use solana_system_interface::instruction::create_account;
use spl_token::{
    id as token_program_id, 
    instruction::{initialize_mint2, mint_to},
    state::Mint,
};
use spl_associated_token_account::get_associated_token_address;

use borsh::BorshSerialize;

use crate::instruction::MovieInstruction;
use crate::state::{ReviewState, ReviewCommentCounterState, ReviewCommentState};
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
        MovieInstruction::AddComment { comment } => {
            process_add_comment(program_id, accounts, comment)
        },
        MovieInstruction::InitializeMint => {
            initialize_token_mint(program_id, accounts)
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
    let accounts_iter = &mut accounts.iter();
    
    let reviewer = next_account_info(accounts_iter)?;
    let movie_review = next_account_info(accounts_iter)?;
    let counter = next_account_info(accounts_iter)?;
    let token_mint = next_account_info(accounts_iter)?;
    let mint_auth = next_account_info(accounts_iter)?;
    let user_ata = next_account_info(accounts_iter)?;
    let system_program = next_account_info(accounts_iter)?;
    let token_program = next_account_info(accounts_iter)?;

    if !reviewer.is_signer {
        return Err(ProgramError::MissingRequiredSignature)
    }

    let (movie_review_pda, movie_review_bump) = Pubkey::find_program_address(
        &[reviewer.key.as_ref(), title.as_bytes().as_ref()], 
        program_id,
    );

    if *movie_review.key != movie_review_pda {
        return Err(ProgramError::InvalidSeeds);
    }

    if rating < 1 || rating > 5 {
        return Err(ReviewError::InvalidRating.into());
    }

    let total_len = ReviewState::space(&title, &description);
    if total_len > ReviewState::MAX_SPACE {
        return Err(ReviewError::InvalidDataLength.into());
    }

    let rent = Rent::get()?;

    let movie_account_rent = rent.minimum_balance(ReviewState::MAX_SPACE);

    invoke_signed(
        &solana_system_interface::instruction::create_account(
            reviewer.key, 
            movie_review.key, 
            movie_account_rent, 
            ReviewState::MAX_SPACE as u64, 
            program_id,
        ), 
        &[
            reviewer.clone(), 
            movie_review.clone(), 
            system_program.clone(),
        ], 
        &[
            &[
                reviewer.key.as_ref(), 
                title.as_bytes().as_ref(),
                &[movie_review_bump],
            ],
        ]
    )?;


    let mut movie_review_account_data = 
        try_from_slice_unchecked::<ReviewState>(&movie_review.data.borrow())?;   

    if movie_review_account_data.is_initialized() {
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    movie_review_account_data.discriminator = ReviewState::DISCRIMINATOR.to_string();
    movie_review_account_data.reviewer = *reviewer.key;
    movie_review_account_data.title = title;
    movie_review_account_data.rating = rating;
    movie_review_account_data.description = description;
    movie_review_account_data.is_initialized = true;

    movie_review_account_data.serialize(&mut &mut movie_review.data.borrow_mut()[..])?;

    let counter_rent = rent.minimum_balance(ReviewCommentCounterState::SPACE);

    let (counter_pda, counter_bump) = Pubkey::find_program_address(
        &[movie_review.key.as_ref(), b"counter"], 
        program_id,
    );

    if *counter.key != counter_pda {
        return Err(ProgramError::InvalidSeeds);
    }

    invoke_signed(
        &solana_system_interface::instruction::create_account(
            reviewer.key, 
            counter.key, 
            counter_rent, 
            ReviewCommentCounterState::SPACE as u64, 
            program_id,
        ), 
        &[
            reviewer.clone(), 
            counter.clone(), 
            system_program.clone(),
        ], 
        &[
            &[
                movie_review.key.as_ref(), b"counter", &[counter_bump],
            ]
        ],
    )?;


    let mut counter_data =
        try_from_slice_unchecked::<ReviewCommentCounterState>(&counter.data.borrow())?;

    if counter_data.is_initialized() {
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    counter_data.discriminator = ReviewCommentCounterState::DISCRIMINATOR.to_string();
    counter_data.counter = 0;
    counter_data.is_initialized = true;

    counter_data.serialize(&mut &mut counter.data.borrow_mut()[..])?;

    let (mint_pda, _mint_bump) = 
        Pubkey::find_program_address(&[b"token_mint"], program_id);
    let (mint_auth_pda, mint_auth_bump) =
        Pubkey::find_program_address(&[b"mint_auth"], program_id);

    if *token_mint.key != mint_pda {
        return Err(ReviewError::IncorrectAccountError.into());
    }

    if *mint_auth.key != mint_auth_pda {
        return Err(ReviewError::IncorrectAccountError.into());
    }

    if *user_ata.key != get_associated_token_address(reviewer.key, token_mint.key) {
        return Err(ReviewError::IncorrectAccountError.into());
    }

    if *token_program.key != token_program_id() {
        return Err(ReviewError::IncorrectAccountError.into());
    }

    invoke_signed(
        &mint_to(
            token_program.key, 
            token_mint.key, 
            user_ata.key, 
            mint_auth.key, 
            &[], 
            10 * LAMPORTS_PER_SOL,
        )?, 
        &[token_mint.clone(), user_ata.clone(), mint_auth.clone()], 
        &[
            &[b"mint_auth", &[mint_auth_bump]]
        ],
    )?;

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

    let mut movie_review_account_data = 
        try_from_slice_unchecked::<ReviewState>(&movie_review_account.data.borrow())?;

    if !movie_review_account_data.is_initialized() {
        return Err(ProgramError::UninitializedAccount);
    }

    if rating < 1 || rating > 5 {
        return Err(ReviewError::InvalidRating.into());
    }

    let total_len = ReviewState::space(&title, &description);
    if total_len > ReviewState::MAX_SPACE {
        return Err(ReviewError::InvalidDataLength.into());
    }

    movie_review_account_data.rating = rating;
    movie_review_account_data.description = description;

    movie_review_account_data.serialize(&mut &mut movie_review_account.data.borrow_mut()[..])?;

    Ok(())
}

pub fn process_add_comment(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    comment: String,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();

    let commenter = next_account_info(accounts_iter)?;
    let movie_review = next_account_info(accounts_iter)?;
    let counter = next_account_info(accounts_iter)?;
    let comment_account = next_account_info(accounts_iter)?;
    let token_mint = next_account_info(accounts_iter)?;
    let mint_auth = next_account_info(accounts_iter)?;
    let user_ata = next_account_info(accounts_iter)?;
    let system_program = next_account_info(accounts_iter)?;
    let token_program = next_account_info(accounts_iter)?;

    let mut counter_data = 
        try_from_slice_unchecked::<ReviewCommentCounterState>(&counter.data.borrow())?;

    let comment_account_space = ReviewCommentState::space(&comment);

    let rent = Rent::get()?;
    let comment_account_rent = rent.minimum_balance(comment_account_space);

    let (comment_pda, comment_pda_bump) = Pubkey::find_program_address(
        &[
            movie_review.key.as_ref(),
            counter_data.counter.to_be_bytes().as_ref(),
        ], 
        program_id,
    );

    if *comment_account.key != comment_pda {
        return Err(ProgramError::InvalidSeeds);
    }

    invoke_signed(
        &solana_system_interface::instruction::create_account(
            commenter.key, 
            comment_account.key, 
            comment_account_rent, 
            comment_account_space as u64, 
            program_id,
        ), 
        &[
            commenter.clone(),
            comment_account.clone(),
            system_program.clone(),
        ], 
        &[
            &[
                movie_review.key.as_ref(),
                counter_data.counter.to_be_bytes().as_ref(),
                &[comment_pda_bump],
            ]
        ],
    )?;

    let mut comment_account_data =
        try_from_slice_unchecked::<ReviewCommentState>(&comment_account.data.borrow())?;

    if comment_account_data.is_initialized() {
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    comment_account_data.discriminator = ReviewCommentState::DISCRIMINATOR.to_string();
    comment_account_data.review = *movie_review.key;
    comment_account_data.commenter = *commenter.key;
    comment_account_data.comment = comment;
    comment_account_data.count = counter_data.counter;
    comment_account_data.is_initialized = true;

    comment_account_data.serialize(&mut &mut comment_account.data.borrow_mut()[..])?;

    counter_data.counter = 
        counter_data.counter.checked_add(1).ok_or(ProgramError::ArithmeticOverflow)?;
        
    counter_data.serialize(&mut &mut counter.data.borrow_mut()[..])?;

    let (mint_pda, _mint_bump) =
        Pubkey::find_program_address(&[b"token_mint"], program_id);
    let (mint_auth_pda, mint_auth_bump) =
        Pubkey::find_program_address(&[b"mint_auth"], program_id);

    if *token_mint.key != mint_pda {
        return Err(ReviewError::IncorrectAccountError.into());
    }

    if *mint_auth.key != mint_auth_pda {
        return Err(ReviewError::IncorrectAccountError.into());
    }
    
    if *user_ata.key != get_associated_token_address(commenter.key, token_mint.key) {
        return Err(ReviewError::IncorrectAccountError.into());
    }

    if *token_program.key != token_program_id() {
        return Err(ReviewError::IncorrectAccountError.into());
    }

    invoke_signed(
        &mint_to(
            token_program.key, 
            token_mint.key, 
            user_ata.key, 
            mint_auth.key, 
            &[], 
            5 * LAMPORTS_PER_SOL
        )?, 
        &[mint_auth.clone(), user_ata.clone(), token_mint.clone()], 
        &[
            &[b"mint_auth", &[mint_auth_bump]],
        ],
    )?;

    Ok(())
}

pub fn initialize_token_mint(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();

    let initializer = next_account_info(accounts_iter)?;
    let token_mint = next_account_info(accounts_iter)?;
    let mint_auth = next_account_info(accounts_iter)?;
    let system_program =next_account_info(accounts_iter)?;
    let token_program = next_account_info(accounts_iter)?;

    let (mint_pda, mint_bump) = 
        Pubkey::find_program_address(&[b"token_mint"], program_id);
    let (mint_auth_pda, _mint_auth_bump) = 
        Pubkey::find_program_address(&[b"mint_auth"], program_id);

    if *token_mint.key != mint_pda {
        return Err(ReviewError::IncorrectAccountError.into());
    }

    if *mint_auth.key != mint_auth_pda {
        return Err(ReviewError::IncorrectAccountError.into());
    }
    
    if *token_program.key != token_program_id() {
        return Err(ReviewError::IncorrectAccountError.into());
    }

    let rent = Rent::get()?;

    let mint_rent = rent.minimum_balance(Mint::LEN);

    invoke_signed(
        &create_account(
            initializer.key, 
            token_mint.key, 
            mint_rent, 
            Mint::LEN as u64, 
            token_program.key,
        ), 
        &[initializer.clone(), token_mint.clone(), system_program.clone()], 
        &[
            &[b"token_mint", &[mint_bump]],
        ],
    )?;

    invoke_signed(
        &initialize_mint2(
            token_program.key, 
            token_mint.key, 
            mint_auth.key, 
            None, 
            9,
        )?, 
        &[token_mint.clone(), mint_auth.clone()], 
        &[
            &[b"token_mint", &[mint_bump]]
        ],
    )?;

    Ok(())
}