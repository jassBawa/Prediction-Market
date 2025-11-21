use anchor_lang::prelude::*;
use anyhow::Result;
use db::{create_market, init_pool};

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
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL")?;
    let program_id = std::env::var("PROGRAM_ID")?;
    let pool = init_pool(&database_url).await?;

    // subscribe
    let ws_client = PubsubClient::new("ws://localhost:8900").await?;

    let config = RpcTransactionLogsConfig { commitment: None };
    let program_id =
        Pubkey::from_str(&program_id).map_err(|e| anyhow::anyhow!("Invalid program id: {}", e))?;

    let filter = RpcTransactionLogsFilter::Mentions(vec![program_id.to_string()]);
    let (mut log_stream, _sub) = ws_client.logs_subscribe(filter, config).await?;

    let discriminator =
        solana_program::hash::hashv(&[b"event:MarketInitialized"]).to_bytes()[..8].to_vec();

    while let Some(msg) = log_stream.next().await {
        for log in msg.value.logs {
            if let Some(stripped) = log.strip_prefix("Program data: ") {
                if let Ok(data) = base64::decode(stripped) {
                    if data.starts_with(&discriminator) {
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
                    }
                }
            }
        }
    }

    Ok(())
}
