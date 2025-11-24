use std::collections::{BTreeMap, VecDeque};

use rust_decimal::Decimal;
use serde::Serialize;
use uuid::Uuid;

use crate::types::{OrderEntry, Side, Trade};

#[derive(Debug, Serialize, Default)]
pub struct OrderBook {
    pub bids: BTreeMap<Decimal, VecDeque<OrderEntry>>,
    pub asks: BTreeMap<Decimal, VecDeque<OrderEntry>>,
}

impl OrderBook {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn best_bid(&self) -> Option<Decimal> {
        self.bids.keys().next_back().copied()
    }

    pub fn best_ask(&self) -> Option<Decimal> {
        self.asks.keys().next().copied()
    }

    pub fn place_order(
        &mut self,
        mut order: OrderEntry,
        side: Side,
    ) -> (Uuid, Vec<Trade>, Decimal) {
        let mut trades: Vec<Trade> = Vec::new();
        println!("placing order from engine");

        match side {
            Side::Bid => self.execute_bid_order(&mut order, &mut trades),
            Side::Ask => self.execute_ask_order(&mut order, &mut trades),
        }

        (order.id, trades, order.qty)
    }

    fn execute_bid_order(&mut self, order: &mut OrderEntry, trades: &mut Vec<Trade>) {
        while order.qty > Decimal::ZERO {
            // Get the best ask price
            let Some(best_ask_price) = self.best_ask() else {
                break;
            };

            // Stop if order price is below market price
            if order.price < best_ask_price {
                break;
            }

            let should_remove = if let Some(queue) = self.asks.get_mut(&best_ask_price) {
                Self::match_orders_static(order, queue, trades, true);
                queue.is_empty()
            } else {
                break;
            };

            // Remove empty price level
            if should_remove {
                self.asks.remove(&best_ask_price);
            } else {
                break;
            }
        }

        // Add remaining quantity to orderbook
        if order.qty > Decimal::ZERO {
            let entry = OrderEntry {
                id: order.id,
                user_id: order.user_id.clone(),
                market_id: order.market_id.clone(),
                price: order.price,
                qty: order.qty,
            };
            self.bids.entry(order.price).or_default().push_back(entry);
        }
    }

    fn execute_ask_order(&mut self, order: &mut OrderEntry, trades: &mut Vec<Trade>) {
        while order.qty > Decimal::ZERO {
            // Get the best bid price
            let Some(best_bid_price) = self.best_bid() else {
                break;
            };

            // Stop if order price is above market price
            if order.price > best_bid_price {
                break;
            }

            // Match against orders at this price level
            let should_remove = if let Some(queue) = self.bids.get_mut(&best_bid_price) {
                Self::match_orders_static(order, queue, trades, false);
                queue.is_empty()
            } else {
                break;
            };

            // Remove empty price level
            if should_remove {
                self.bids.remove(&best_bid_price);
            } else {
                break;
            }
        }

        // Add remaining quantity to orderbook
        if order.qty > Decimal::ZERO {
            let entry = OrderEntry {
                id: order.id,
                user_id: order.user_id.clone(),
                market_id: order.market_id.clone(),
                price: order.price,
                qty: order.qty,
            };
            self.asks.entry(order.price).or_default().push_back(entry);
        }
    }

    fn match_orders_static(
        taker: &mut OrderEntry,
        maker_queue: &mut VecDeque<OrderEntry>,
        trades: &mut Vec<Trade>,
        taker_is_buyer: bool,
    ) {
        while taker.qty > Decimal::ZERO && !maker_queue.is_empty() {
            let maker = maker_queue.front_mut().unwrap();
            let matched_qty = taker.qty.min(maker.qty);

            maker.qty -= matched_qty;
            taker.qty -= matched_qty;

            // Record the trade
            trades.push(Trade {
                market_id: maker.market_id.clone(),
                buyer_id: if taker_is_buyer {
                    taker.user_id.clone()
                } else {
                    maker.user_id.clone()
                },
                seller_id: if taker_is_buyer {
                    maker.user_id.clone()
                } else {
                    taker.user_id.clone()
                },
                price: taker.price,
                quantity: matched_qty,
            });

            // Remove maker if fully filled
            if maker.qty == Decimal::ZERO {
                maker_queue.pop_front();
            }
        }
    }

    pub fn cancel_order(&mut self, side: Side, price: Decimal, order_id: Uuid) -> (bool, String) {
        let map = match side {
            Side::Bid => &mut self.bids,
            Side::Ask => &mut self.asks,
        };

        if let Some(queue) = map.get_mut(&price) {
            if let Some(pos) = queue.iter().position(|o| o.id == order_id) {
                queue.remove(pos);
                if queue.is_empty() {
                    map.remove(&price);
                }
                return (true, "Order cancelled successfully".to_string());
            }
        }

        (false, "Order not found".to_string())
    }
}
