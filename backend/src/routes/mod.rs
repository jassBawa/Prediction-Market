use axum::{
    Router, middleware,
    routing::{get, post},
};

use crate::{AppState, auth::middelware::auth_middleware};

pub mod market;
pub mod orders;

pub fn create_router(state: AppState) -> Router {
    let public_routes = Router::new()
        .route("/markets", get(market::list_markets_handler))
        .route("/markets/{address}", get(market::get_market_handler));

    let protected_routes = Router::new()
        .route("/orders/open", post(orders::open_order_handler))
        .route("/orders/close", post(orders::close_order_handler))
        .layer(middleware::from_fn(auth_middleware));

    public_routes.merge(protected_routes).with_state(state)
}
