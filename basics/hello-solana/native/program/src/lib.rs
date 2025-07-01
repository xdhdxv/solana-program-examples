use solana_program::{
    entrypoint,
    entrypoint::ProgramResult,
    pubkey::Pubkey,
    account_info::AccountInfo,
    msg,
};

entrypoint!(process_instruction);

pub fn process_instruction(
    _program_id: &Pubkey,
    _accounts: &[AccountInfo],
    _instruction_data: &[u8],
) -> ProgramResult {
    msg!("Hello, Solana!");

    Ok(())
}

#[cfg(test)]
mod test {
    use anyhow::Result;

    use solana_program_test::*;

    use solana_sdk::{
        pubkey::Pubkey,
        signature::Signer,
        instruction::Instruction,
        transaction::Transaction,
    };

    use super::*;

    #[tokio::test]
    async fn test_hello_solana() -> Result<()> {
        let program_id = Pubkey::new_unique();
        let (banks_client, payer, recent_blockhash) = ProgramTest::new(
            "program", 
            program_id, 
            processor!(process_instruction),
        )
        .start().await;

        let instruction = Instruction::new_with_borsh(
            program_id, 
            &(), 
            vec![],
        );

        let transaction = Transaction::new_signed_with_payer(
            &[instruction], 
            Some(&payer.pubkey()), 
            &[&payer], 
            recent_blockhash,
        );

        let transaction_result = 
            banks_client.process_transaction(transaction).await;

        assert!(transaction_result.is_ok());
        
        Ok(())
    }
}