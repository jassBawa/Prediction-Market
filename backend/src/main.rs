use axum::http::{
    header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE},
    HeaderName, HeaderValue, Method,
};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;

use backend::{config, routes, solana::client::SolanaClient, state};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = config::Config::from_env();

    let db_pool = db::init_pool(&config.database_url)
        .await
        .expect("Failed to create database pool");

    let solana_client = SolanaClient::new(
        std::env::var("SOLANA_RPC_URL")
            .unwrap_or_else(|_| "https://api.devnet.solana.com".to_string()),
        "CxTfQPv74sJmzzTkSQvM569yJGmCpW7T3xjMub2DQrwW".to_string(),
        std::env::var("SOLANA_KEYPAIR_PATH").unwrap_or_else(|_| {
            "../solana-prediction-market-program/target/deploy/predix_program-keypair.json"
                .to_string()
        }),
    );

    let app_state = state::AppState {
        markets: Arc::new(RwLock::new(HashMap::new())),
        db: db_pool,
        rpc: Arc::new(solana_client),
    };

    let privy_header = HeaderName::from_static("privy-id-token");
    let cors = CorsLayer::new()
        .allow_origin("http://localhost:3000".parse::<HeaderValue>()?)
        .allow_methods([Method::GET, Method::POST, Method::DELETE, Method::PUT])
        .allow_headers([AUTHORIZATION, ACCEPT, CONTENT_TYPE, privy_header])
        .allow_credentials(true);

    let app = routes::create_router(app_state).layer(cors);

    let listener = tokio::net::TcpListener::bind(&config.server_addr)
        .await
        .expect("Failed to bind to address");

    axum::serve(listener, app).await.expect("Server error");
    Ok(())
}
