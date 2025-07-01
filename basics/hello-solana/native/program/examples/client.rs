use anyhow::{anyhow, Result};

use solana_client::nonblocking::rpc_client::RpcClient;

use solana_sdk::{
    commitment_config::CommitmentConfig,
    signature::{Signer, Keypair, keypair},
    native_token::LAMPORTS_PER_SOL,
    instruction::Instruction,
    transaction::Transaction,
};

#[tokio::main]
async fn main() -> Result<()> {
    let program_id = keypair::read_keypair_file("target/deploy/program-keypair.json")
        .map_err(|e| anyhow!("{e}"))?.pubkey();

    let client = RpcClient::new_with_commitment(
        "http://localhost:8899".to_string(), 
        CommitmentConfig::confirmed(),
    );
    let recent_blockhash = client.get_latest_blockhash().await?;

    let fee_payer = Keypair::new();

    let airdrop_signature = client.request_airdrop(
        &fee_payer.pubkey(), 
        LAMPORTS_PER_SOL,
    ).await?;
    client.poll_for_signature(&airdrop_signature).await?;


    let ix = Instruction::new_with_borsh(
        program_id, 
        &(), 
        vec![],
    );

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&fee_payer.pubkey()), 
        &[&fee_payer], 
        recent_blockhash,
    );

    let tx_signature = 
        client.send_and_confirm_transaction_with_spinner(&tx).await?;

    println!("tx signature: {}", tx_signature);
    
    Ok(())
}