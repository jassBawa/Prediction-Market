use anchor_client::solana_sdk::pubkey::Pubkey;
use anyhow::{self, Result};
use client::SolanaClient;
use matching_engine::{ShareType, Trade};
use solana_client::rpc_client::RpcClient;
use std::str::FromStr;

use crate::solana::{
    accounts::get_ata_address,
    market::{derive::derive_share_mints, fetch::fetch_market},
    utils::decimal_to_lamports,
};

pub mod accounts;
pub mod client;
pub mod market;
pub mod trading;
pub mod transactions;
pub mod types;
pub mod utils;

pub use accounts::verify_delegation;
pub use transactions::{
    generate_approval_transaction, generate_merge_transaction, generate_split_transaction,
};

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
    println!("  → Fetching market mints for {:?} shares...", share_type);
    let (collateral_mint, share_mint) = get_market_mints(
        &client.rpc_url(),
        &client.program_id(),
        market_address,
        share_type,
    )
    .await?;

    println!("  → Converting {} trades to match fills...", trades.len());
    let match_fills = convert_trades_to_match_fills(&trades, share_type)?;

    println!("  → Building transaction accounts...");
    let remaining_accounts = build_remaining_accounts(&trades, &collateral_mint, &share_mint)?;

    println!("  → Sending transaction to Solana...");
    let signature = client
        .execute_match_multi(market_address, match_fills, remaining_accounts)
        .await?;

    Ok(signature)
}

fn convert_trades_to_match_fills(
    trades: &[Trade],
    share_type: ShareType,
) -> Result<Vec<types::MatchFill>, Box<dyn std::error::Error>> {
    let mut match_fills = Vec::new();

    for (i, trade) in trades.iter().enumerate() {
        let shares = decimal_to_lamports(trade.quantity)?;
        let total_collateral = trade.quantity * trade.price;
        let price = decimal_to_lamports(total_collateral)?;

        println!(
            "  → Fill[{}]: shares={}, price={}, side={:?}",
            i, shares, price, share_type
        );
        println!(
            "     buyer={}, seller={}",
            &trade.buyer_id[..8],
            &trade.seller_id[..8]
        );

        match_fills.push(types::MatchFill {
            shares,
            price,
            side: types::share_type_to_trade_side(share_type),
        });
    }

    Ok(match_fills)
}

fn build_remaining_accounts(
    trades: &[Trade],
    collateral_mint: &str,
    share_mint: &str,
) -> Result<Vec<types::AccountMeta>, Box<dyn std::error::Error>> {
    let mut accounts = Vec::new();
    let collateral_mint_pubkey = Pubkey::from_str(collateral_mint)?;
    let share_mint_pubkey = Pubkey::from_str(share_mint)?;

    for trade in trades {
        let buyer = Pubkey::from_str(&trade.buyer_id)?;
        let seller = Pubkey::from_str(&trade.seller_id)?;

        let buyer_collateral = get_ata_address(&buyer, &collateral_mint_pubkey);
        let seller_collateral = get_ata_address(&seller, &collateral_mint_pubkey);
        let buyer_share = get_ata_address(&buyer, &share_mint_pubkey);
        let seller_share = get_ata_address(&seller, &share_mint_pubkey);

        accounts.push(types::AccountMeta::new(buyer_collateral.to_string()));
        accounts.push(types::AccountMeta::new(seller_collateral.to_string()));
        accounts.push(types::AccountMeta::new(buyer_share.to_string()));
        accounts.push(types::AccountMeta::new(seller_share.to_string()));
        accounts.push(types::AccountMeta::new_readonly(buyer.to_string()));
        accounts.push(types::AccountMeta::new_readonly(seller.to_string()));
    }

    Ok(accounts)
}
