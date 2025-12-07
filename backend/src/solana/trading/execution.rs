use anchor_client::solana_sdk::pubkey::Pubkey;
use anyhow::{self, Result};
use matching_engine::Trade;
use std::str::FromStr;

use crate::solana::{client::SolanaClient, types::TradeSide};

use super::conversion::{build_remaining_accounts, convert_trades_to_match_fills};

pub async fn execute_trades_on_chain(
    client: &SolanaClient,
    market_address: &str,
    trades: Vec<Trade>,
    share_type: TradeSide,
    market_id: u64,
    collateral_mint: String,
) -> Result<String, Box<dyn std::error::Error>> {
    let program_pubkey = Pubkey::from_str(client.program_id())?;
    let collateral_mint_pubkey = Pubkey::from_str(&collateral_mint)?;

    let match_fills = convert_trades_to_match_fills(&trades, share_type)?;

    let remaining_accounts = build_remaining_accounts(
        &trades,
        share_type,
        market_id,
        &program_pubkey,
        collateral_mint_pubkey,
    );

    let signature = client
        .execute_match_multi(market_address, match_fills, remaining_accounts)
        .await?;

    Ok(signature)
}
