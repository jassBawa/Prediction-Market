use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::prelude::FromRow;

#[derive(Debug, FromRow)]
pub struct MarketRow {
    pub id: i64,
    pub market_id: String,
    pub market_address: String,
    pub creator_address: String,
    pub program_id: String,

    pub title: String,
    pub description: Option<String>,
    pub category: Option<String>,

    pub yes_option: String,
    pub no_option: String,

    pub end_timestamp: i64,
    pub created_slot: i64,

    pub collateral_mint: String,
    pub collateral_vault: String,
    pub yes_mint: String,
    pub no_mint: String,
    pub bump: i16,

    pub resolved: bool,
    pub resolved_outcome: Option<String>,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Market {
    pub id: i64,
    pub market_id: String,
    pub market_address: String,
    pub creator_address: String,
    pub program_id: String,

    pub title: String,
    pub description: Option<String>,
    pub category: Option<String>,

    pub yes_option: String,
    pub no_option: String,

    pub end_timestamp: i64,
    pub created_slot: i64,

    pub collateral_mint: String,
    pub collateral_vault: String,
    pub yes_mint: String,
    pub no_mint: String,
    pub bump: i16,

    pub resolved: bool,
    pub resolved_outcome: Option<String>,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<MarketRow> for Market {
    fn from(row: MarketRow) -> Self {
        Self {
            id: row.id,
            market_id: row.market_id,
            market_address: row.market_address,
            creator_address: row.creator_address,
            program_id: row.program_id,

            title: row.title,
            description: row.description,
            category: row.category,

            yes_option: row.yes_option,
            no_option: row.no_option,

            end_timestamp: row.end_timestamp,
            created_slot: row.created_slot,

            collateral_mint: row.collateral_mint,
            collateral_vault: row.collateral_vault,
            yes_mint: row.yes_mint,
            no_mint: row.no_mint,
            bump: row.bump,

            resolved: row.resolved,
            resolved_outcome: row.resolved_outcome,

            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}
