use anchor_client::solana_sdk::pubkey::Pubkey;
use anyhow::{self, Result};
use matching_engine::{ShareType, Trade};
use solana_client::rpc_client::RpcClient;
use std::str::FromStr;

use crate::solana::{
    client::SolanaClient,
    market::{derive::derive_share_mints, fetch::fetch_market},
};

use super::conversion::{build_remaining_accounts, convert_trades_to_match_fills};

async fn get_market_mints(
    rpc_url: &str,
    program_id: &str,
    market_address: &str,
    share_type: ShareType,
) -> Result<(String, String), Box<dyn std::error::Error>> {
    let rpc = RpcClient::new(rpc_url);
    let market_pubkey = Pubkey::from_str(market_address)?;
    let program_pubkey = Pubkey::from_str(program_id)?;

    let market = fetch_market(&rpc, &market_pubkey)?;

    let market_id = market.market_id;
    let collateral_mint = market.collateral_mint;

    let (yes_mint, no_mint) = derive_share_mints(&program_pubkey, market_id);

    let share_mint = match share_type {
        ShareType::Yes => yes_mint,
        ShareType::No => no_mint,
    };

    Ok((collateral_mint.to_string(), share_mint.to_string()))
}

pub async fn execute_trades_on_chain(
    client: &SolanaClient,
    market_address: &str,
    trades: Vec<Trade>,
    share_type: ShareType,
) -> Result<String, Box<dyn std::error::Error>> {
    let (collateral_mint, share_mint) = get_market_mints(
        &client.rpc_url(),
        &client.program_id(),
        market_address,
        share_type,
    )
    .await?;

    let match_fills = convert_trades_to_match_fills(&trades, share_type)?;

    println!("Building transaction accounts...");
    let remaining_accounts = build_remaining_accounts(&trades, &collateral_mint, &share_mint)?;

    println!("Sending transaction to Solana...");
    let signature = client
        .execute_match_multi(market_address, match_fills, remaining_accounts)
        .await?;

    Ok(signature)
}
