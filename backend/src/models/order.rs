use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub use matching_engine::{ShareType, Side, Trade};

#[derive(Deserialize, Debug)]
pub struct PlaceOrderReq {
    pub user_id: String,
    pub market_id: String,
    pub side: Side,
    pub share: ShareType,
    pub price: String,
    pub qty: String,
}

#[derive(Serialize, Debug)]
pub struct PlaceOrderRes {
    pub order_id: Uuid,
    pub trades: Vec<Trade>,
    pub remaining_qty: Decimal,
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
