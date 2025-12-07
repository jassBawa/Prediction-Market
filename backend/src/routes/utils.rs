use axum::http::StatusCode;
// use db::get_market_by_address;
use matching_engine::{run_market_engine, EngineMsg};
use rust_decimal::Decimal;
use solana_sdk::pubkey::Pubkey;
use tokio::sync::mpsc;

use std::str::FromStr;

use crate::state::Shared;

pub fn parse_decimal(value: &str, field_name: &str) -> Result<Decimal, (StatusCode, String)> {
    Decimal::from_str(value).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            format!("Invalid {}: {}", field_name, value),
        )
    })
}

pub fn parse_pubkey(value: &str, field_name: &str) -> Result<Pubkey, (StatusCode, String)> {
    Pubkey::from_str(value).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            format!("Invalid {}: {}", field_name, e),
        )
    })
}

pub async fn get_or_create_market_engine(
    state: &Shared,
    market_id: &str,
    _market_address: &str,
) -> Result<mpsc::Sender<EngineMsg>, (StatusCode, String)> {
    let markets = state.markets.read().await;

    if let Some(tx) = markets.get(market_id) {
        return Ok(tx.clone());
    }

    drop(markets);

    // TODO: find good approach for this (dont' remove)
    // let _market = get_market_by_address(&state.db, market_address)
    //     .await
    //     .map_err(|e| {
    //         eprintln!("Database error fetching market: {}", e);
    //         (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e))
    //     })?
    //     .ok_or_else(|| {
    //         eprintln!("Market not found: {}", market_address);
    //         (StatusCode::NOT_FOUND, "Market not found".to_string())
    //     })?;

    let mut markets = state.markets.write().await;

    if let Some(tx) = markets.get(market_id) {
        Ok(tx.clone())
    } else {
        let (tx, rx) = mpsc::channel::<EngineMsg>(100);
        tokio::spawn(run_market_engine(rx));
        markets.insert(market_id.to_string(), tx.clone());
        Ok(tx)
    }
}

pub async fn get_market_engine(
    state: &Shared,
    market_id: &str,
) -> Result<mpsc::Sender<EngineMsg>, (StatusCode, String)> {
    let markets = state.markets.read().await;

    let market = markets.get(market_id).cloned();
    market.ok_or((StatusCode::NOT_FOUND, "market not found".into()))
}
