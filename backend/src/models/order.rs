use matching_engine::SnapshotData;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub use matching_engine::{ShareType, Side, Trade};

#[derive(Deserialize, Debug)]
pub struct PlaceOrderReq {
    pub market_id: String,
    pub market_address: String,
    pub side: Side,
    pub share: ShareType,
    pub price: String,
    pub qty: String,
}

#[derive(Serialize, Debug)]
#[serde(tag = "status")]
pub enum PlaceOrderRes {
    #[serde(rename = "success")]
    Success {
        order_id: Uuid,
        trades: Vec<Trade>,
        remaining_qty: Decimal,
    },
    #[serde(rename = "delegation_required")]
    DelegationRequired {
        tx_message: String,
        recent_blockhash: String,
    },
}

#[derive(Deserialize, Debug)]
pub struct CancelReq {
    pub market_id: String,
    pub order_id: Uuid,
    pub side: Side,
    pub share: ShareType,
    pub price: Decimal,
}

#[derive(Serialize, Debug)]
pub struct CancelRes {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct OrderBookSide {
    pub bids: Vec<SnapshotData>,
    pub asks: Vec<SnapshotData>,
}

#[derive(Debug, Serialize)]
pub struct OrderBookResponse {
    pub yes: OrderBookSide,
    pub no: OrderBookSide,
}
