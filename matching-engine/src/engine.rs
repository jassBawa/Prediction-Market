use rust_decimal::Decimal;
use tokio::sync::{mpsc, oneshot};
use uuid::Uuid;

use crate::{
    market::MarketBooks,
    types::{FullMarketSnapshot, OrderEntry, ShareType, Side, Trade},
};

pub enum EngineMsg {
    PlaceOrder {
        side: Side,
        share: ShareType,
        order: OrderEntry,
        resp: oneshot::Sender<(Uuid, Vec<Trade>, Decimal)>,
    },
    CloseOrder {
        side: Side,
        share: ShareType,
        price: Decimal,
        order_id: Uuid,
        resp: oneshot::Sender<(bool, String)>,
    },
    Snapshot {
        resp: oneshot::Sender<FullMarketSnapshot>,
    },
}

pub async fn run_market_engine(mut rx: mpsc::Receiver<EngineMsg>) {
    let mut book = MarketBooks::new();

    while let Some(msg) = rx.recv().await {
        match msg {
            EngineMsg::PlaceOrder {
                side,
                share,
                order,
                resp,
            } => {
                let result = match share {
                    ShareType::Yes => book.yes.place_order(order, side),
                    ShareType::No => book.no.place_order(order, side),
                };
                let _ = resp.send(result);
            }

            EngineMsg::CloseOrder {
                side,
                share,
                price,
                order_id,
                resp,
            } => {
                let result = match share {
                    ShareType::Yes => book.yes.cancel_order(side, price, order_id),
                    ShareType::No => book.no.cancel_order(side, price, order_id),
                };
                let _ = resp.send(result);
            }

            EngineMsg::Snapshot { resp } => {
                let snapshot = book.snapshot();
                let _ = resp.send(snapshot);
            }
        }
    }
}
