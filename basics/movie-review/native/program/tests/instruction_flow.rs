use anyhow::Result;
use borsh::BorshSerialize;

use solana_program_test::*;

use solana_sdk::{
    pubkey::Pubkey, 
    signature::{Signer, Keypair}, 
    instruction::{Instruction, AccountMeta},
    transaction::Transaction,
    borsh1::try_from_slice_unchecked,
};

use solana_system_interface::program::id as system_program_id;

use program::processor::process_instruction;
use program::state::MovieAccountState;

#[tokio::test]
async fn add_movie_review_ix_test() -> Result<()> {
    let program_id = Pubkey::new_unique();

    let (banks_client, payer, recent_blockhash) = ProgramTest::new(
        "program", 
        program_id, 
        processor!(process_instruction)
    ).start().await;

    let movie_title = String::from("Interstellar");
    let movie_rating = 5;
    let movie_description = String::from(
        "Sometimes I just need to see the start. Or end. Or a trailer. 
        Or the music and theme from Hans Zimmer. Or the whole movie. 
        Just to feel that thing, I only get from this movie. 
        That the earth, space and time are something special, mystical"
    );

    let (movie_review_account, _bump) = Pubkey::find_program_address(
        &[payer.pubkey().as_ref(), movie_title.as_bytes().as_ref()], 
        &program_id,
    );

    println!("Testing add movie instruction...");

    let movie_review_payload = MovieReviewPayload {
        title: movie_title.clone(),
        rating: movie_rating,
        description: movie_description.clone()
    };

    let mut add_movie_instruction_data = vec![0];

    movie_review_payload.serialize(&mut add_movie_instruction_data)?;

    let add_movie_review_ix = Instruction::new_with_bytes(
        program_id, 
        &add_movie_instruction_data, 
        vec![
            AccountMeta::new(
                payer.pubkey(), 
                true,
            ),
            AccountMeta::new(
                movie_review_account, 
                false,
            ),
            AccountMeta::new_readonly(
                system_program_id(), 
                false,
            ),
        ],
    );

    let add_movie_review_tx = Transaction::new_signed_with_payer(
        &[add_movie_review_ix], 
        Some(&payer.pubkey()), 
        &[&payer], 
        recent_blockhash,
    );

    let add_movie_review_tx_result = banks_client.process_transaction(add_movie_review_tx).await;

    assert!(add_movie_review_tx_result.is_ok());

    let movie_review_account_state = 
        banks_client.get_account(movie_review_account).await?.unwrap();

    let movie_review_account_state = 
        try_from_slice_unchecked::<MovieAccountState>(&movie_review_account_state.data)?;

    assert_eq!(movie_review_account_state.is_initialized, true);
    assert_eq!(movie_review_account_state.rating, movie_rating);
    assert_eq!(movie_review_account_state.title, movie_title);
    assert_eq!(movie_review_account_state.description, movie_description);

    Ok(())
}

#[tokio::test]
async fn add_movie_review_ix_with_invalid_movie_review_account_test() -> Result<()> {
    let program_id = Pubkey::new_unique();

    let (banks_client, payer, recent_blockhash) = ProgramTest::new(
        "program", 
        program_id, 
        processor!(process_instruction)
    ).start().await;

    let movie_title = String::from("Interstellar");
    let movie_rating = 5;
    let movie_description = String::from(
        "Sometimes I just need to see the start. Or end. Or a trailer. 
        Or the music and theme from Hans Zimmer. Or the whole movie. 
        Just to feel that thing, I only get from this movie. 
        That the earth, space and time are something special, mystical"
    );

    let another_reviewer = Keypair::new();

    let (movie_review_account, _bump) = Pubkey::find_program_address(
        &[another_reviewer.pubkey().as_ref(), movie_title.as_bytes().as_ref()], 
        &program_id,
    );

    println!("Testing add movie instruction...");

    let movie_review_payload = MovieReviewPayload {
        title: movie_title.clone(),
        rating: movie_rating,
        description: movie_description.clone()
    };

    let mut add_movie_instruction_data = vec![0];

    movie_review_payload.serialize(&mut add_movie_instruction_data)?;

    let add_movie_review_ix = Instruction::new_with_bytes(
        program_id, 
        &add_movie_instruction_data, 
        vec![
            AccountMeta::new(
                payer.pubkey(), 
                true,
            ),
            AccountMeta::new(
                movie_review_account, 
                false,
            ),
            AccountMeta::new_readonly(
                system_program_id(), 
                false,
            ),
        ],
    );

    let add_movie_review_tx = Transaction::new_signed_with_payer(
        &[add_movie_review_ix], 
        Some(&payer.pubkey()), 
        &[&payer], 
        recent_blockhash,
    );

    let add_movie_review_tx_result = banks_client.process_transaction(add_movie_review_tx).await;

    assert!(add_movie_review_tx_result.is_err());

    Ok(())
}

#[tokio::test]
async fn update_movie_review_ix_test() -> Result<()> {
    let program_id = Pubkey::new_unique();

    let (banks_client, payer, recent_blockhash) = ProgramTest::new(
        "program", 
        program_id, 
        processor!(process_instruction)
    ).start().await;

    let movie_title = String::from("Interstellar");
    let movie_rating = 5;
    let movie_description = String::from(
        "Sometimes I just need to see the start. Or end. Or a trailer. 
        Or the music and theme from Hans Zimmer. Or the whole movie. 
        Just to feel that thing, I only get from this movie. 
        That the earth, space and time are something special, mystical"
    );

    let (movie_review_account, _bump) = Pubkey::find_program_address(
        &[payer.pubkey().as_ref(), movie_title.as_bytes().as_ref()], 
        &program_id,
    );

    println!("Testing add movie review instruction...");

    let movie_review_payload = MovieReviewPayload {
        title: movie_title.clone(),
        rating: movie_rating,
        description: movie_description.clone()
    };

    let mut add_movie_instruction_data = vec![0];

    movie_review_payload.serialize(&mut add_movie_instruction_data)?;

    let add_movie_review_ix = Instruction::new_with_bytes(
        program_id, 
        &add_movie_instruction_data, 
        vec![
            AccountMeta::new(
                payer.pubkey(), 
                true,
            ),
            AccountMeta::new(
                movie_review_account, 
                false,
            ),
            AccountMeta::new_readonly(
                system_program_id(), 
                false,
            ),
        ],
    );

    let add_movie_review_tx = Transaction::new_signed_with_payer(
        &[add_movie_review_ix], 
        Some(&payer.pubkey()), 
        &[&payer], 
        recent_blockhash,
    );

    let add_movie_review_tx_result = banks_client.process_transaction(add_movie_review_tx).await;

    assert!(add_movie_review_tx_result.is_ok());

    let movie_review_account_state = 
        banks_client.get_account(movie_review_account).await?.unwrap();

    let movie_review_account_state = 
        try_from_slice_unchecked::<MovieAccountState>(&movie_review_account_state.data)?;

    assert_eq!(movie_review_account_state.is_initialized, true);
    assert_eq!(movie_review_account_state.rating, movie_rating);
    assert_eq!(movie_review_account_state.title, movie_title);
    assert_eq!(movie_review_account_state.description, movie_description);

    let movie_title = String::from("Interstellar");
    let new_movie_rating = 3;
    let new_movie_description = String::from("Not bad.");

    let (movie_review_account, _bump) = Pubkey::find_program_address(
        &[payer.pubkey().as_ref(), movie_title.as_bytes().as_ref()], 
        &program_id,
    );

    println!("Testing update movie review instruction...");

    let movie_review_payload = MovieReviewPayload {
        title: movie_title.clone(),
        rating: new_movie_rating,
        description: new_movie_description.clone(),
    };

    let mut update_movie_review_ix_data = vec![1];

    movie_review_payload.serialize(&mut update_movie_review_ix_data)?;

    let update_movie_review_ix = Instruction::new_with_bytes(
        program_id, 
        &update_movie_review_ix_data, 
        vec![
            AccountMeta::new(
                payer.pubkey(), 
                true,
            ),
            AccountMeta::new(
                movie_review_account,
                false,
            ),
        ],
    );

    let update_movie_review_tx = Transaction::new_signed_with_payer(
        &[update_movie_review_ix], 
        Some(&payer.pubkey()), 
        &[payer], 
        recent_blockhash,
    );

    let update_movie_review_tx_result = 
        banks_client.process_transaction(update_movie_review_tx).await;

    assert!(update_movie_review_tx_result.is_ok());

    let movie_review_account_state = 
        banks_client.get_account(movie_review_account).await?.unwrap();

    let movie_review_account_state = 
        try_from_slice_unchecked::<MovieAccountState>(&movie_review_account_state.data)?;

    assert_eq!(movie_review_account_state.is_initialized, true);
    assert_eq!(movie_review_account_state.rating, new_movie_rating);
    assert_eq!(movie_review_account_state.title, movie_title);
    assert_eq!(movie_review_account_state.description, new_movie_description);

    Ok(())
}

#[derive(BorshSerialize)]
struct MovieReviewPayload {
    title: String,
    rating: u8,
    description: String,
}