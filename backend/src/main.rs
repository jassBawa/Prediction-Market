use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;

// Import all modules from lib
use backend::{config, routes, state};

#[tokio::main]
async fn main() {
    let config = config::Config::from_env();

    let db_pool = db::init_pool(&config.database_url)
        .await
        .expect("Failed to create database pool");

    let app_state = state::AppState {
        markets: Arc::new(RwLock::new(HashMap::new())),
        db: db_pool,
    };

    let app = routes::create_router(app_state);

    let listener = tokio::net::TcpListener::bind(&config.server_addr)
        .await
        .expect("Failed to bind to address");

    axum::serve(listener, app).await.expect("Server error");
}
