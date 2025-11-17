pub mod engine;
pub mod market;
pub mod orderbook;
pub mod types;

// Re-export main types for convenience
pub use engine::{run_market_engine, EngineMsg};
pub use market::MarketBooks;
pub use orderbook::OrderBook;
pub use types::{
    FullMarketSnapshot, OrderBookSnapshot, OrderEntry, ShareType, Side, SnapshotData, Trade,
};
