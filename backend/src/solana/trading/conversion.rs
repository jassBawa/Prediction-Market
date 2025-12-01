use crate::solana::{
    accounts::get_ata_address,
    types::{share_type_to_trade_side, AccountMeta, MatchFill},
    utils::decimal_to_lamports,
};
use matching_engine::{ShareType, Trade};
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

pub fn convert_trades_to_match_fills(
    trades: &[Trade],
    share_type: ShareType,
) -> Result<Vec<MatchFill>, Box<dyn std::error::Error>> {
    let mut match_fills = Vec::new();

    for (i, trade) in trades.iter().enumerate() {
        let shares = decimal_to_lamports(trade.quantity)?;
        let total_collateral = trade.quantity * trade.price;
        let price = decimal_to_lamports(total_collateral)?;

        println!(
            "  → Fill[{}]: shares={}, price={}, side={:?}",
            i, shares, price, share_type
        );
        println!(
            "     buyer={}, seller={}",
            &trade.buyer_id[..8],
            &trade.seller_id[..8]
        );

        match_fills.push(MatchFill {
            shares,
            price,
            side: share_type_to_trade_side(share_type),
        });
    }

    Ok(match_fills)
}

pub fn build_remaining_accounts(
    trades: &[Trade],
    collateral_mint: &str,
    share_mint: &str,
) -> Result<Vec<AccountMeta>, Box<dyn std::error::Error>> {
    let mut accounts = Vec::new();
    let collateral_mint_pubkey = Pubkey::from_str(collateral_mint)?;
    let share_mint_pubkey = Pubkey::from_str(share_mint)?;

    for trade in trades {
        let buyer = Pubkey::from_str(&trade.buyer_id)?;
        let seller = Pubkey::from_str(&trade.seller_id)?;

        let buyer_collateral = get_ata_address(&buyer, &collateral_mint_pubkey);
        let seller_collateral = get_ata_address(&seller, &collateral_mint_pubkey);
        let buyer_share = get_ata_address(&buyer, &share_mint_pubkey);
        let seller_share = get_ata_address(&seller, &share_mint_pubkey);

        accounts.push(AccountMeta::new(buyer_collateral.to_string()));
        accounts.push(AccountMeta::new(seller_collateral.to_string()));
        accounts.push(AccountMeta::new(buyer_share.to_string()));
        accounts.push(AccountMeta::new(seller_share.to_string()));
        accounts.push(AccountMeta::new_readonly(buyer.to_string()));
        accounts.push(AccountMeta::new_readonly(seller.to_string()));
    }

    Ok(accounts)
}
