use crate::{
    orderbook::OrderBook,
    types::{FullMarketSnapshot, SnapshotData},
};

pub struct MarketBooks {
    pub yes: OrderBook,
    pub no: OrderBook,
}

impl MarketBooks {
    pub fn new() -> Self {
        Self {
            yes: OrderBook::new(),
            no: OrderBook::new(),
        }
    }

    pub fn snapshot(&self) -> FullMarketSnapshot {
        let yes_bids = self.snapshot_side(&self.yes.bids);
        let yes_asks = self.snapshot_side(&self.yes.asks);
        let no_bids = self.snapshot_side(&self.no.bids);
        let no_asks = self.snapshot_side(&self.no.asks);

        ((yes_bids, yes_asks), (no_bids, no_asks))
    }

    fn snapshot_side(
        &self,
        side: &std::collections::BTreeMap<
            rust_decimal::Decimal,
            std::collections::VecDeque<crate::types::OrderEntry>,
        >,
    ) -> Vec<SnapshotData> {
        side.iter()
            .rev()
            .map(|(price, orders)| {
                let quantity: rust_decimal::Decimal = orders.iter().map(|o| o.qty).sum();
                let total = quantity * price;
                SnapshotData {
                    price: *price,
                    quantity,
                    total,
                }
            })
            .collect()
    }
}

impl Default for MarketBooks {
    fn default() -> Self {
        Self::new()
    }
}
