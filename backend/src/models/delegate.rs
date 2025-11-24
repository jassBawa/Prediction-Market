use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct ApproveRequest {
    pub market_id: String,
    pub mint: String,
    pub user_ata: String,
    pub program_id: String,
    pub amount: u64,
    pub decimals: u8,
}

#[derive(Debug, Serialize)]
pub struct ApproveRes {
    pub tx_message: String,
    pub recent_blockhash: String,
}
