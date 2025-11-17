use std::sync::Arc;

use axum::{
    routing::{get, post},
    Router,
};

use crate::state::AppState;

pub mod market;
pub mod orders;

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/markets", get(market::list_markets_handler))
        .route("/markets/{address}", get(market::get_market_handler))
        .route("/orders/open", post(orders::place_order))
        .route("/orders/close", post(orders::cancel_order))
        .route("/orderbook/{market_id}", get(orders::get_orderbook))
        // .layer(middleware::from_fn(auth_middleware))
        .with_state(Arc::new(state))
}
