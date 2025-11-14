use anyhow::Result;
use axum::{Router, routing::get};
use db::{DbPool, get_db_pool};
use privy_rs::PrivyClient;
use std::net::SocketAddr;
use tokio::net::TcpListener;

mod auth;
mod routes;

#[derive(Clone)]
pub struct AppState {
    pub db: DbPool,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    let db = get_db_pool().await?;

    // let app_id = std::env::var("PRIVY_APP_ID").expect("PRIVY_APP_ID environment variable not set");
    // let app_secret =
    //     std::env::var("PRIVY_APP_SECRET").expect("PRIVY_APP_SECRET environment variable not set");

    let state = AppState { db };

    let api = routes::create_router(state);

    let app = Router::new().nest("/api/v1", api);

    let addr: SocketAddr = "0.0.0.0:3000".parse()?;
    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
