use solana_program::{
    program_pack::{IsInitialized, Sealed},
    pubkey::Pubkey,
};

use borsh::{BorshSerialize, BorshDeserialize};

#[derive(BorshSerialize, BorshDeserialize)]
pub struct ReviewState {
    pub discriminator: String,
    pub is_initialized: bool,
    pub reviewer: Pubkey,
    pub rating: u8,
    pub title: String,
    pub description: String,
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct ReviewCommentCounterState {
    pub discriminator: String,
    pub is_initialized: bool,
    pub counter: u64,
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct ReviewCommentState {
    pub discriminator: String,
    pub is_initialized: bool,
    pub review: Pubkey,
    pub commenter: Pubkey,
    pub comment: String,
    pub count: u64,
}

impl ReviewState {
    pub const DISCRIMINATOR: &'static str = "review";
    pub const MAX_SPACE: usize = 1000;

    pub fn space(title: &str, description: &str) -> usize {
        (4 + Self::DISCRIMINATOR.len())
        + 1
        + 32
        + 1
        + (4 + title.len())
        + (4 + description.len())
    }
}

impl ReviewCommentCounterState {
    pub const DISCRIMINATOR: &'static str = "counter";
    pub const SPACE: usize = (4 + Self::DISCRIMINATOR.len()) + 1 + 8;
}

impl ReviewCommentState {
    pub const DISCRIMINATOR: &'static str = "comment";

    pub fn space(comment: &str) -> usize {
        (4 + Self::DISCRIMINATOR.len())
        + 1
        + 32
        + 32
        + (4 + comment.len())
        + 8
    }
}

impl Sealed for ReviewState {}

impl IsInitialized for ReviewState {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

impl IsInitialized for ReviewCommentCounterState {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

impl IsInitialized for ReviewCommentState {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}
