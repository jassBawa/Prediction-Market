use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub server_addr: String,
}

impl Config {
    pub fn from_env() -> Self {
        dotenvy::dotenv().ok();

        let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

        let server_addr = env::var("SERVER_ADDR").unwrap_or_else(|_| "0.0.0.0:3000".to_string());

        Self {
            database_url,
            server_addr,
        }
    }
}
