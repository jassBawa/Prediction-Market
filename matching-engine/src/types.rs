use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize, Serialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Bid,
    Ask,
}

#[derive(Deserialize, Serialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShareType {
    Yes,
    No,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct OrderEntry {
    pub id: Uuid,
    pub user_id: String,
    pub market_id: String,
    pub price: Decimal,
    pub qty: Decimal,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Trade {
    pub market_id: String,
    pub buyer_id: String,
    pub seller_id: String,
    pub price: Decimal,
    pub quantity: Decimal,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SnapshotData {
    pub price: Decimal,
    pub quantity: Decimal,
    pub total: Decimal,
}

pub type OrderBookSnapshot = (Vec<SnapshotData>, Vec<SnapshotData>);

pub type FullMarketSnapshot = (OrderBookSnapshot, OrderBookSnapshot);

#[derive(Debug, Serialize, Deserialize)]
pub struct MarketSnapshot {
    pub yes: OrderBookSnapshot,
    pub no: OrderBookSnapshot,
}
