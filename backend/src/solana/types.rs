// use borsh::{BorshDeserialize, BorshSerialize};

use matching_engine::ShareType;
pub use predix_program::{MatchFill, TradeSide};

#[derive(Debug, Clone)]
pub struct AccountMeta {
    pub pubkey: String,
    pub is_writable: bool,
    pub is_signer: bool,
}

impl AccountMeta {
    pub fn new(pubkey: String) -> Self {
        Self {
            pubkey,
            is_writable: true,
            is_signer: false,
        }
    }

    pub fn new_readonly(pubkey: String) -> Self {
        Self {
            pubkey,
            is_writable: false,
            is_signer: false,
        }
    }
}

pub fn share_type_to_trade_side(share_type: ShareType) -> TradeSide {
    match share_type {
        ShareType::Yes => TradeSide::Yes,
        ShareType::No => TradeSide::No,
    }
}
