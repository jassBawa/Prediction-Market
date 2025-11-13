use anchor_lang::prelude::*;
use anyhow::Result;
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
    // subscribe
    let ws_client = PubsubClient::new("ws://localhost:8900").await?;

    let config = RpcTransactionLogsConfig { commitment: None };
    let program_id = Pubkey::from_str("8TzgHbENzybwKDterDa57kzPM6RfjLWQdQCqYPW9mZ3X").unwrap();

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
                            Ok(event) => println!("EVENT: {:?}", event),
                            Err(e) => eprintln!("Failed to parse event: {:?}", e),
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
