use matching_engine::{ShareType, Side};
use rust_decimal::Decimal;
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

use crate::solana::{
    accounts::get_ata_address,
    client::SolanaClient,
    market::{derive_market_pda, derive_share_mint, fetch_market},
    utils::decimal_to_lamports,
};

pub async fn verify_delegation(
    client: &SolanaClient,
    market_address: &str,
    user_wallet: &str,
    side: Side,
    share_type: ShareType,
    price: Decimal,
    qty: Decimal,
) -> Result<bool, Box<dyn std::error::Error>> {
    let rpc = RpcClient::new(client.rpc_url());
    let program_pubkey = Pubkey::from_str(client.program_id())?;
    let market_pubkey = Pubkey::from_str(market_address)?;
    let user_pubkey = Pubkey::from_str(user_wallet)?;

    let market = fetch_market(&rpc, &market_pubkey)?;
    let market_id = market.market_id;
    let collateral_mint = market.collateral_mint;

    let (market_pda, _bump) = derive_market_pda(&program_pubkey, market_id);

    if market_pubkey != market_pda {
        return Err(anyhow::anyhow!(
            "Market address {} does not match calculated PDA {} for market_id {}",
            market_pubkey,
            market_pda,
            market_id
        )
        .into());
    }

    let market_pda = market_pda;

    println!("=== DELEGATION VERIFICATION DEBUG ===");
    println!("Market address: {}", market_address);
    println!("Market ID: {}", market_id);
    println!("Program ID: {}", program_pubkey);
    println!("Calculated market_pda: {}", market_pda);

    let (token_account_pubkey, required_amount_lamports) = match side {
        Side::Bid => {
            let collateral_ata = get_ata_address(&user_pubkey, &collateral_mint);
            let required = decimal_to_lamports(qty * price)?;
            (collateral_ata, required as u64)
        }
        Side::Ask => {
            let share_mint = derive_share_mint(&program_pubkey, market_id, share_type);
            let share_ata = get_ata_address(&user_pubkey, &share_mint);
            let required = decimal_to_lamports(qty * price)?;
            (share_ata, required as u64)
        }
    };

    let account = match rpc.get_account(&token_account_pubkey) {
        Ok(acc) => acc,
        Err(_) => {
            return Ok(false);
        }
    };

    let account_data = account.data;

    if account_data.len() < 165 {
        return Err(anyhow::anyhow!("Invalid token account data length").into());
    };

    let delegate_byte = account_data[72];
    let delegate = if delegate_byte == 1 {
        Some(
            Pubkey::try_from(&account_data[73..105])
                .map_err(|_| anyhow::anyhow!("Invalid delegate pubkey"))?,
        )
    } else {
        None
    };

    let delegated_amount = u64::from_le_bytes(
        account_data[105..113]
            .try_into()
            .map_err(|_| anyhow::anyhow!("Invalid delegated_amount"))?,
    );

    println!("delegated_amount = {}", delegated_amount);
    println!("required = {}", required_amount_lamports);

    struct TokenAccountInfo {
        delegate: Option<Pubkey>,
        delegated_amount: u64,
    }

    let token_account_info = TokenAccountInfo {
        delegate,
        delegated_amount,
    };

    match token_account_info.delegate {
        Some(delegate) if delegate == market_pda => {
            println!("Delegate matches market_pda! Checking amounts...");
            let sufficient = token_account_info.delegated_amount >= required_amount_lamports;
            println!(
                "Delegated: {}, Required: {}, Sufficient: {}",
                token_account_info.delegated_amount, required_amount_lamports, sufficient
            );
            Ok(sufficient)
        }
        Some(_delegate) => {
            println!("   But you need to delegate to: {}", market_pda);
            println!();
            Ok(false)
        }
        None => {
            println!("❌ No delegate set in token account");
            Ok(false)
        }
    }
}
