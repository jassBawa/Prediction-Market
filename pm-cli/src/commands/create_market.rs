use anchor_client::solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_program,
};
use anchor_lang::declare_program;
use anyhow::Result;
use predix_program::{client::accounts, client::args};
use std::rc::Rc;

declare_program!(predix_program);
use crate::utils::create_mint::create_mint;

pub async fn create_market(
    program: anchor_client::Program<Rc<Keypair>>,
    payer_keypair: Keypair,
    market_id: u64,
    metadata: String,
    end_time: i64,
) -> Result<()> {
    println!("Creating market with ID: {}", market_id);

    // Create RPC client for create_mint function
    let rpc_client =
        solana_client::nonblocking::rpc_client::RpcClient::new("http://127.0.0.1:8899".to_string());

    println!("Creating collateral mint...");
    let collateral_mint = create_mint(&rpc_client, &payer_keypair, payer_keypair.pubkey()).await?;
    println!("Collateral mint created: {}", collateral_mint);

    let (market_pda, _market_bump) =
        Pubkey::find_program_address(&[b"market", &market_id.to_le_bytes()], &predix_program::ID);

    let (vault_pda, _vault_bump) = Pubkey::find_program_address(
        &[b"collateral_vault", &market_id.to_le_bytes()],
        &predix_program::ID,
    );

    let (yes_mint_pda, _yes_bump) = Pubkey::find_program_address(
        &[b"yes_mint", &market_id.to_le_bytes()],
        &predix_program::ID,
    );

    let (no_mint_pda, _no_bump) =
        Pubkey::find_program_address(&[b"no_mint", &market_id.to_le_bytes()], &predix_program::ID);

    println!("Market PDA: {}", market_pda);
    println!("Vault PDA: {}", vault_pda);
    println!("YES mint PDA: {}", yes_mint_pda);
    println!("NO mint PDA: {}", no_mint_pda);
    println!("\nInitializing market...");

    let initialize_tx = program
        .request()
        .accounts(accounts::InitializeMarket {
            market: market_pda,
            vault: vault_pda,
            collateral_mint,
            yes_mint: yes_mint_pda,
            no_mint: no_mint_pda,
            admin: payer_keypair.pubkey(),
            system_program: system_program::ID,
            token_program: anchor_spl::token::ID,
            associated_token_program: anchor_spl::associated_token::ID,
        })
        .args(args::InitializeMarket {
            market_id,
            metadata: metadata.clone(),
            expiration_timestamp: end_time,
        })
        .send()
        .await?;

    println!("\nMarket created successfully!");
    println!("Transaction signature: {}", initialize_tx);
    println!("Market address: {}", market_pda);
    println!("Metadata: {}", metadata);
    Ok(())
}
