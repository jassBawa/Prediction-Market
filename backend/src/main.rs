use anyhow::Result;
use axum::{Router, routing::get};
use db::{DbPool, get_db_pool};
use std::net::SocketAddr;
use tokio::net::TcpListener;

mod routes;

#[derive(Clone)]
pub struct AppState {
    pub db: DbPool,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    let db = get_db_pool().await?;
    let state = AppState { db };

    let api_routes = Router::new().route("/markets", get(routes::market::list_markets_handler));

    let app = Router::new().nest("/api/v1", api_routes).with_state(state);

    let addr: SocketAddr = "0.0.0.0:3000".parse()?;
    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
