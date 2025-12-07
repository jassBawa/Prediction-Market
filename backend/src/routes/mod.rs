use std::sync::Arc;

use axum::{
    middleware,
    routing::{get, post},
    Router,
};

use crate::{
    auth::middleware::auth_middleware,
    state::{AppState, Shared},
};

pub mod market;
pub mod orders;
pub mod utils;

pub fn create_router(state: AppState) -> Router {
    let shared: Shared = Arc::new(state);

    let public_router = Router::new()
        .route("/markets", get(market::list_markets_handler))
        .route("/markets/{address}", get(market::get_market_handler))
        .with_state(shared.clone());

    let protected_router = Router::new()
        .route("/orders/open", post(orders::place_order))
        .route("/orders/close", post(orders::cancel_order))
        .route("/orderbook/{market_id}", get(orders::get_orderbook))
        .route("/orders/split", post(orders::split_order))
        .route("/orders/merge", post(orders::merge_order))
        .layer(middleware::from_fn(auth_middleware))
        .with_state(shared.clone());

    public_router.merge(protected_router)
}
