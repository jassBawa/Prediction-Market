use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;

use crate::state::Shared;
use db::{
    get_market_by_address, list_active_markets, list_markets, list_resolved_markets, models::Market,
};

#[derive(Debug, Deserialize, Default)]
pub struct MarketQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub resolved: Option<bool>,
}

pub async fn list_markets_handler(
    State(state): State<Shared>,
    Query(params): Query<MarketQuery>,
) -> Result<Json<Vec<Market>>, StatusCode> {
    let limit = params.limit.unwrap_or(20).clamp(1, 100);
    let offset = params.offset.unwrap_or(0).max(0);

    let markets_result = match params.resolved {
        Some(true) => list_resolved_markets(&state.db).await,
        Some(false) => list_active_markets(&state.db).await,
        None => list_markets(&state.db, limit, offset).await,
    };

    markets_result
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

pub async fn get_market_handler(
    State(state): State<Shared>,
    Path(address): Path<String>,
) -> Result<Json<Option<Market>>, StatusCode> {
    get_market_by_address(&state.db, &address)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}
