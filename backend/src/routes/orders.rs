use axum::{
    extract::{Extension, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::{auth::claims::PrivyClaims, AppState};

#[derive(Debug, Deserialize)]
pub struct OrderRequest {
    pub market_address: String,
    pub amount: u64,
    pub side: String,
}

#[derive(Debug, Serialize)]
pub struct OrderResponse {
    pub message: String,
    pub market_address: String,
    pub signer: String,
}

pub async fn open_order_handler(
    State(_state): State<AppState>,
    Extension(claims): Extension<PrivyClaims>,
    Json(payload): Json<OrderRequest>,
) -> Result<Json<OrderResponse>, StatusCode> {
    let response = OrderResponse {
        message: format!(
            "Opened {} order for {} units",
            payload.side, payload.amount
        ),
        market_address: payload.market_address,
        signer: claims.sub.clone(),
    };

    Ok(Json(response))
}

pub async fn close_order_handler(
    State(_state): State<AppState>,
    Extension(claims): Extension<PrivyClaims>,
    Json(payload): Json<OrderRequest>,
) -> Result<Json<OrderResponse>, StatusCode> {
    let response = OrderResponse {
        message: format!(
            "Closed {} order for {} units",
            payload.side, payload.amount
        ),
        market_address: payload.market_address,
        signer: claims.sub.clone(),
    };

    Ok(Json(response))
}

