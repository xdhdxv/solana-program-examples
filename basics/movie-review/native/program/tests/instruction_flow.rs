use anyhow::Result;
use borsh::BorshSerialize;

use solana_program_test::*;

use solana_sdk::{
    instruction::Instruction, pubkey::Pubkey, signer::Signer, transaction::Transaction
};

use program::*;

#[tokio::test]
async fn instruction_flow_test() -> Result<()> {
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

    println!("Testing add movie instruction...");

    let movie_review_payload = MovieReviewPayload {
        title: movie_title,
        rating: movie_rating,
        description: movie_description
    };

    let mut add_movie_instruction_data = vec![0];

    movie_review_payload.serialize(&mut add_movie_instruction_data);

    let add_movie_review_ix = Instruction::new_with_bytes(
        program_id, 
        &add_movie_instruction_data, 
        vec![],
    );

    let add_movie_review_tx = Transaction::new_signed_with_payer(
        &[add_movie_review_ix], 
        Some(&payer.pubkey()), 
        &[&payer], 
        recent_blockhash,
    );

    let add_movie_review_tx_result = banks_client.process_transaction(add_movie_review_tx).await;

    assert!(add_movie_review_tx_result.is_ok());

    Ok(())
}

#[derive(BorshSerialize)]
struct MovieReviewPayload {
    title: String,
    rating: u8,
    description: String,
}