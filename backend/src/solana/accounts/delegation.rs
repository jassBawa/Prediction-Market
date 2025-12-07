use anchor_lang::AnchorDeserialize;
use matching_engine::{ShareType, Side};
use rust_decimal::Decimal;
use solana_client::rpc_client::RpcClient;
use solana_sdk::program_pack::Pack;
use solana_sdk::pubkey::Pubkey;
use spl_token::state::Account as TokenAccount;
use std::str::FromStr;

use crate::solana::{
    accounts::get_ata_address, client::SolanaClient, market::derive_share_mint, types::Market,
    utils::decimal_to_lamports,
};

pub struct MarketData {
    pub market_id: u64,
    pub collateral_mint: Pubkey,
    pub program_id: Pubkey,
}

pub async fn verify_delegation(
    client: &SolanaClient,
    market_address: &str,
    user_wallet: &str,
    side: Side,
    share_type: ShareType,
    price: Decimal,
    qty: Decimal,
    market_data: Option<MarketData>,
) -> Result<bool, Box<dyn std::error::Error>> {
    let rpc = RpcClient::new(client.rpc_url());
    let market_pubkey = Pubkey::from_str(market_address)?;
    let user_pubkey = Pubkey::from_str(user_wallet)?;

    let (market_id, collateral_mint, actual_program_id) = if let Some(data) = market_data {
        (data.market_id, data.collateral_mint, data.program_id)
    } else {
        let market_account = rpc.get_account(&market_pubkey)?;
        let actual_program_id = market_account.owner;

        let mut data = &market_account.data[8..];
        let market = Market::deserialize(&mut data)
            .map_err(|e| anyhow::anyhow!("Failed to deserialize market: {}", e))?;

        (market.market_id, market.collateral_mint, actual_program_id)
    };

    let (token_account_pubkey, required_amount_lamports) = match side {
        Side::Bid => {
            let collateral_ata = get_ata_address(&user_pubkey, &collateral_mint);
            let required = decimal_to_lamports(qty * price)?;
            (collateral_ata, required as u64)
        }
        Side::Ask => {
            let share_mint = derive_share_mint(&actual_program_id, market_id, share_type);
            let share_ata = get_ata_address(&user_pubkey, &share_mint);
            let required = decimal_to_lamports(qty)?;
            (share_ata, required as u64)
        }
    };

    let account = match rpc.get_account(&token_account_pubkey) {
        Ok(acc) => acc,
        Err(_) => {
            return Ok(false);
        }
    };

    let token_state: TokenAccount = TokenAccount::unpack(&account.data)
        .map_err(|_| anyhow::anyhow!("Failed to unpack token account"))?;

    let delegate: Option<Pubkey> = token_state.delegate.into();
    match delegate {
        Some(d) if d == market_pubkey => {
            Ok(token_state.delegated_amount >= required_amount_lamports)
        }
        _ => Ok(false),
    }
}
