#[allow(deprecated)]
use anchor_client::solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_program,
};
use anchor_spl::associated_token::get_associated_token_address;
use anyhow::Result;
use std::rc::Rc;

use crate::types::{accounts, args};

pub async fn create_market(
    program: anchor_client::Program<Rc<Keypair>>,
    payer_keypair: Keypair,
    market_id: u64,
    metadata: String,
    end_time: i64,
) -> Result<()> {
    let collateral_mint = "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU"
        .parse::<Pubkey>()
        .expect("Invalid USDC Mint");

    let (market_pda, _market_bump) = Pubkey::find_program_address(
        &[b"market", &market_id.to_le_bytes()],
        &predix_program::id(),
    );

    let vault_pda = get_associated_token_address(&market_pda, &collateral_mint);

    let (yes_mint_pda, _yes_bump) = Pubkey::find_program_address(
        &[b"yes_mint", &market_id.to_le_bytes()],
        &predix_program::id(),
    );

    let (no_mint_pda, _no_bump) = Pubkey::find_program_address(
        &[b"no_mint", &market_id.to_le_bytes()],
        &predix_program::id(),
    );

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
