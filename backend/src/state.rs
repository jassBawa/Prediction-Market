use std::{collections::HashMap, sync::Arc};

use tokio::sync::{mpsc, RwLock};
use matching_engine::EngineMsg;

#[derive(Clone)]
pub struct AppState {
    pub markets: Arc<RwLock<HashMap<String, mpsc::Sender<EngineMsg>>>>,
    pub db: db::DbPool,
}

pub type Shared = Arc<AppState>;
