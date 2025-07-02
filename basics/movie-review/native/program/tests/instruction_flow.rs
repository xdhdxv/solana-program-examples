use anyhow::Result;

use solana_program_test::*;

use solana_sdk::{
    address_lookup_table::instruction, pubkey::Pubkey
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

    let add_movie_review_instruction_data = MovieReviewPayload {
        title: movie_title,
        rating: movie_rating,
        description: movie_description
    };

}