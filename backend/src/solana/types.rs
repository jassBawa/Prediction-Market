use anchor_lang::declare_program;
use matching_engine::ShareType;

// Declare the program once here - this generates the predix_program module
declare_program!(predix_program);

// Re-export commonly used types
pub use predix_program::{
    accounts::Market,
    client::{accounts, args},
    types::{MatchFill, TradeSide},
};

// Conversion function from matching-engine ShareType to on-chain TradeSide
pub fn share_type_to_trade_side(share_type: ShareType) -> TradeSide {
    match share_type {
        ShareType::Yes => TradeSide::Yes,
        ShareType::No => TradeSide::No,
    }
}
