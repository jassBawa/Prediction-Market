use anchor_lang::prelude::*;
use anyhow::Result;
use db::{create_market, init_pool, update_market_resolution};

use base64;
use solana_client::{
    nonblocking::pubsub_client::PubsubClient,
    rpc_config::{RpcTransactionLogsConfig, RpcTransactionLogsFilter},
};
use std::str::FromStr;

use tokio_stream::StreamExt;

#[derive(Debug, AnchorDeserialize)]
pub struct MarketInitialized {
    pub market: Pubkey,
    pub authority: Pubkey,
    pub market_id: u64,
    pub metadata: String,
    pub collateral_vault: Pubkey,
    pub collateral_mint: Pubkey,
    pub yes_mint: Pubkey,
    pub no_mint: Pubkey,
    pub expiration_timestamp: i64,
    pub bump: u8,
}

#[derive(Debug, AnchorDeserialize)]
pub struct MarketSettled {
    pub market: Pubkey,
    pub market_id: u64,
    pub outcome: u8,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL")?;
    let program_id = std::env::var("PROGRAM_ID")?;
    let pool = init_pool(&database_url).await?;

    // subscribe
    let ws_client = PubsubClient::new("wss://api.devnet.solana.com/").await?;

    let config = RpcTransactionLogsConfig { commitment: None };
    let program_id =
        Pubkey::from_str(&program_id).map_err(|e| anyhow::anyhow!("Invalid program id: {}", e))?;

    let filter = RpcTransactionLogsFilter::Mentions(vec![program_id.to_string()]);
    let (mut log_stream, _sub) = ws_client.logs_subscribe(filter, config).await?;

    let initialized_discriminator =
        solana_program::hash::hashv(&[b"event:MarketInitialized"]).to_bytes()[..8].to_vec();
    let settle_discriminator =
        solana_program::hash::hashv(&[b"event:MarketSettled"]).to_bytes()[..8].to_vec();

    while let Some(msg) = log_stream.next().await {
        for log in msg.value.logs {
            println!("{:?}", log);
            if let Some(stripped) = log.strip_prefix("Program data: ") {
                #[allow(deprecated)]
                if let Ok(data) = base64::decode(stripped) {
                    println!("{:?}", data);
                    if data.starts_with(&initialized_discriminator) {
                        let payload = &data[8..];
                        match MarketInitialized::try_from_slice(payload) {
                            Ok(event) => {
                                println!("EVENT: {:?}", event);
                                let market_address = event.market.to_string();
                                let market_id = event.market_id.to_string();
                                let creator_address = event.authority.to_string();
                                let program_id_str = program_id.to_string();
                                let title = event.metadata.clone();
                                let yes_option = "YES".to_string();
                                let no_option = "NO".to_string();
                                let collateral_mint = event.collateral_mint.to_string();
                                let collateral_vault = event.collateral_vault.to_string();
                                let yes_mint = event.yes_mint.to_string();
                                let no_mint = event.no_mint.to_string();
                                let bump = event.bump as i16;

                                match create_market(
                                    &pool,
                                    &market_id,
                                    &market_address,
                                    &creator_address,
                                    &program_id_str,
                                    &title,
                                    None,
                                    None,
                                    &yes_option,
                                    &no_option,
                                    event.expiration_timestamp,
                                    event.market_id as i64,
                                    &collateral_mint,
                                    &collateral_vault,
                                    &yes_mint,
                                    &no_mint,
                                    bump,
                                )
                                .await
                                {
                                    Ok(id) => {
                                        println!("Market inserted into DB with id {}", id);
                                    }
                                    Err(e) => {
                                        eprintln!(
                                            "Error inserting market {}: {}",
                                            market_address, e
                                        );
                                    }
                                }
                            }
                            Err(e) => eprintln!("Failed to parse event: {:?}", e),
                        }
                    } else if data.starts_with(&settle_discriminator) {
                        let payload = &data[8..];
                        match MarketSettled::try_from_slice(payload) {
                            Ok(event) => {
                                println!("EVENT: {:?}", event);
                                let market_address = event.market.to_string();

                                let resolved_outcome = match event.outcome {
                                    0 => Some("Yes".to_string()),
                                    1 => Some("No".to_string()),
                                    _ => {
                                        eprintln!("WARNING: Market {} settled with Undecided outcome (2). This shouldn't happen!",
                                               market_address);
                                        None
                                    }
                                };

                                match update_market_resolution(
                                    &pool,
                                    &market_address,
                                    &resolved_outcome.clone().unwrap_or_else(|| "No".to_string()),
                                )
                                .await
                                {
                                    Ok(_) => {
                                        println!(
                                            "Market {} settled in DB with outcome: {:?}",
                                            market_address, resolved_outcome
                                        );
                                    }
                                    Err(e) => {
                                        eprintln!(
                                            "Error updating market settlement {}: {}",
                                            market_address, e
                                        );
                                    }
                                }
                            }
                            Err(e) => eprintln!("Failed to parse event: {:?}", e),
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
