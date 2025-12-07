use crate::solana::{
    accounts::get_ata_address,
    types::{MatchFill, TradeSide},
    utils::decimal_to_lamports,
};
use anchor_lang::prelude::AccountMeta;
use matching_engine::Trade;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

pub fn convert_trades_to_match_fills(
    trades: &[Trade],
    share_type: TradeSide,
) -> Result<Vec<MatchFill>, Box<dyn std::error::Error>> {
    let mut match_fills = Vec::new();

    for trade in trades.iter() {
        let shares = decimal_to_lamports(trade.quantity)?;
        let price = decimal_to_lamports(trade.price)?; // Price per share, not total

        println!(
            "  Fill: {} shares at {} (buyer: ...{}, seller: ...{})",
            trade.quantity,
            trade.price,
            &trade.buyer_id[trade.buyer_id.len() - 8..],
            &trade.seller_id[trade.seller_id.len() - 8..]
        );

        match_fills.push(MatchFill {
            shares,
            price,
            side: share_type,
        });
    }

    Ok(match_fills)
}

pub fn build_remaining_accounts(
    trades: &[Trade],
    share_type: TradeSide,
    market_id: u64,
    program_id: &Pubkey,
    collateral_mint: Pubkey,
) -> Vec<AccountMeta> {
    use crate::solana::market::derive_share_mints;

    let mut accounts = Vec::new();

    let (yes_mint, no_mint) = derive_share_mints(program_id, market_id);

    let share_mint = match share_type {
        TradeSide::Yes => yes_mint,
        TradeSide::No => no_mint,
    };

    for trade in trades {
        let buyer = Pubkey::from_str(&trade.buyer_id).expect("Invalid buyer address");
        let seller = Pubkey::from_str(&trade.seller_id).expect("Invalid seller address");

        if buyer == seller {
            panic!("Cannot execute trade: buyer and seller are the same wallet. Self-trading is not allowed.");
        }

        let buyer_collateral = get_ata_address(&buyer, &collateral_mint);
        let seller_collateral = get_ata_address(&seller, &collateral_mint);
        let buyer_share = get_ata_address(&buyer, &share_mint);
        let seller_share = get_ata_address(&seller, &share_mint);

        if buyer_collateral == seller_collateral {
            panic!("Internal error: buyer and seller have the same collateral ATA");
        }

        if buyer_share == seller_share {
            panic!("Internal error: buyer and seller have the same share ATA");
        }

        accounts.push(AccountMeta {
            pubkey: buyer_collateral,
            is_signer: false,
            is_writable: true,
        });
        accounts.push(AccountMeta {
            pubkey: seller_collateral,
            is_signer: false,
            is_writable: true,
        });
        accounts.push(AccountMeta {
            pubkey: buyer_share,
            is_signer: false,
            is_writable: true,
        });
        accounts.push(AccountMeta {
            pubkey: seller_share,
            is_signer: false,
            is_writable: true,
        });
        accounts.push(AccountMeta {
            pubkey: buyer,
            is_signer: false,
            is_writable: false,
        });
        accounts.push(AccountMeta {
            pubkey: seller,
            is_signer: false,
            is_writable: false,
        });
    }

    accounts
}
