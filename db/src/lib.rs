use anyhow::Result;
use sqlx::{Pool, Postgres};

pub type DbPool = Pool<Postgres>;

pub mod models;
pub mod queries;

/// Initialize a database pool from a connection string
pub async fn init_pool(db_url: &str) -> Result<DbPool> {
    let pool = Pool::<Postgres>::connect(db_url).await?;
    sqlx::migrate!("./migrations").run(&pool).await?;
    Ok(pool)
}

/// Get a database pool from the DATABASE_URL environment variable
pub async fn get_db_pool() -> Result<DbPool> {
    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    init_pool(&db_url).await
}

pub use models::{Market, MarketRow};
pub use queries::market::{
    create_market, get_market_by_address, get_market_by_id, list_active_markets, list_markets,
    list_resolved_markets, update_market_resolution,
};
