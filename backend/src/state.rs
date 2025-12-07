use std::{collections::HashMap, sync::Arc};

use matching_engine::EngineMsg;
use tokio::sync::{mpsc, RwLock};

use crate::solana::client::SolanaClient;

#[derive(Clone)]
pub struct AppState {
    pub markets: Arc<RwLock<HashMap<String, mpsc::Sender<EngineMsg>>>>,
    pub db: db::DbPool,
    pub rpc: Arc<SolanaClient>,
}

pub type Shared = Arc<AppState>;
