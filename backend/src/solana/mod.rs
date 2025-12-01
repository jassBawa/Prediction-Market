use anchor_client::{solana_sdk::pubkey::Pubkey, Client};
use anyhow::{self, Result};
use client::SolanaClient;
use matching_engine::{ShareType, Side, Trade};
use rust_decimal::Decimal;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    message::v0::Message as V0Message,
    message::{Message, VersionedMessage},
    signature::Keypair,
    signer::Signer,
    transaction::{Transaction, VersionedTransaction},
};
use spl_token::instruction::approve_checked;
use std::{str::FromStr, sync::Arc};

use crate::solana::{
    accounts::get_ata_address,
    market::{
        derive::derive_share_mints, derive_market_pda, derive_share_mint, fetch::fetch_market,
    },
    utils::{decimal_to_lamports, detect_cluster, load_fee_payer, serialize_transaction},
};

pub mod accounts;
pub mod client;
pub mod market;
pub mod trading;
pub mod types;
pub mod utils;

pub use accounts::verify_delegation;

pub async fn generate_approval_transaction(
    client: &SolanaClient,
    market_address: &str,
    user_wallet: &str,
    side: Side,
    share_type: ShareType,
    price: Decimal,
    qty: Decimal,
) -> Result<(String, String), Box<dyn std::error::Error>> {
    let rpc = RpcClient::new(client.rpc_url());
    let program_pubkey = Pubkey::from_str(client.program_id())?;
    let user_pubkey = Pubkey::from_str(user_wallet)?;
    let market_pubkey = Pubkey::from_str(market_address)?;

    let market = fetch_market(&rpc, &market_pubkey)?;

    let market_id = market.market_id;
    let collateral_mint = market.collateral_mint;

    let (mint_pubkey, user_ata_pubkey, required_amount_lamports) = match side {
        Side::Bid => {
            let collateral_ata = get_ata_address(&user_pubkey, &collateral_mint);
            let required = decimal_to_lamports(qty * price)?;
            (collateral_mint, collateral_ata, required as u64)
        }
        Side::Ask => {
            let share_mint = derive_share_mint(&program_pubkey, market_id, share_type);
            let share_ata = get_ata_address(&user_pubkey, &share_mint);
            let required = decimal_to_lamports(qty)?;
            (share_mint, share_ata, required as u64)
        }
    };

    // Get fee payer
    let fee_payer = load_fee_payer()?;

    let recent_blockhash = rpc
        .get_latest_blockhash()
        .map_err(|e| anyhow::anyhow!("RPC error: {}", e))?;

    println!(
        "Recent blockhash generated at: {:?}",
        std::time::SystemTime::now()
    );
    println!("Blockhash: {} (expires in ~60 seconds)", recent_blockhash);

    let (market_pda, _bump) = derive_market_pda(&program_pubkey, market_id);

    if market_pda != market_pubkey {
        return Err(anyhow::anyhow!(
            "Market PDA mismatch! Calculated: {}, Market address: {}. This suggests market_id {} doesn't match market address.",
            market_pda, market_pubkey, market_id
        ).into());
    }

    println!("=== GENERATING APPROVAL TRANSACTION ===");
    println!("Market address: {}", market_address);
    println!("Market ID: {}", market_id);
    println!("Program ID: {}", program_pubkey);
    println!("Calculated market_pda (delegate): {}", market_pda);
    println!("Market PDA bump: {}", _bump);
    println!("Required amount: {} lamports", required_amount_lamports);
    println!("User ATA: {}", user_ata_pubkey);
    println!("Mint: {}", mint_pubkey);

    let approve_ix = approve_checked(
        &spl_token::id(),
        &user_ata_pubkey,
        &mint_pubkey,
        &market_pda,
        &user_pubkey,
        &[],
        required_amount_lamports,
        6,
    )
    .map_err(|e| anyhow::anyhow!("Failed to create approve instruction: {}", e))?;

    if approve_ix.accounts.len() >= 4 {
        let delegate_in_instruction = approve_ix.accounts[2].pubkey;
        println!("   Delegate in instruction: {}", delegate_in_instruction);
        println!("   Expected market_pda: {}", market_pda);
        if delegate_in_instruction != market_pda {
            return Err(anyhow::anyhow!(
                "CRITICAL BUG: Approve instruction delegate {} doesn't match market_pda {}! This transaction would delegate to the wrong address!",
                delegate_in_instruction, market_pda
            ).into());
        }
        println!("   Delegate matches market_pda correctly!");
    } else {
        return Err(anyhow::anyhow!(
            "Invalid approve_checked instruction: expected at least 4 accounts, got {}",
            approve_ix.accounts.len()
        )
        .into());
    }

    // Build VersionedTransaction using V0Message
    let v0_msg = V0Message::try_compile(&fee_payer.pubkey(), &[approve_ix], &[], recent_blockhash)
        .map_err(|e| anyhow::anyhow!("Failed to compile V0 message: {}", e))?;

    // Wrap V0Message in VersionedMessage
    let versioned_msg = VersionedMessage::V0(v0_msg);

    // Create unsigned VersionedTransaction first
    let num_signatures = versioned_msg.header().num_required_signatures as usize;
    let mut tx = VersionedTransaction {
        signatures: vec![solana_sdk::signature::Signature::default(); num_signatures],
        message: versioned_msg,
    };

    // Partially sign with fee payer only
    // The fee payer should be the first signer (position 0)
    let message_bytes = tx.message.serialize();
    let fee_payer_signature = fee_payer
        .try_sign_message(&message_bytes)
        .map_err(|e| anyhow::anyhow!("Failed to sign with fee payer: {}", e))?;

    // Place the fee payer's signature in position 0
    tx.signatures[0] = fee_payer_signature;

    // Serialize VersionedTransaction
    let (tx_base64, recent_blockhash_str) = serialize_transaction(&tx, &recent_blockhash)?;
    println!("Transaction generated successfully!");

    println!("   Base64 length: {} chars", tx_base64.len());
    println!("   Recent blockhash: {}", recent_blockhash_str);
    println!();
    Ok((tx_base64, recent_blockhash_str))
}

async fn get_market_mints(
    rpc_url: &str,
    program_id: &str,
    market_address: &str,
    share_type: ShareType,
) -> Result<(String, String), Box<dyn std::error::Error>> {
    let rpc = RpcClient::new(rpc_url);
    let market_pubkey = Pubkey::from_str(market_address)?;
    let program_pubkey = Pubkey::from_str(program_id)?;

    let market = fetch_market(&rpc, &market_pubkey)?;

    let market_id = market.market_id;
    let collateral_mint = market.collateral_mint;

    let (yes_mint, no_mint) = derive_share_mints(&program_pubkey, market_id);

    let share_mint = match share_type {
        ShareType::Yes => yes_mint,
        ShareType::No => no_mint,
    };

    Ok((collateral_mint.to_string(), share_mint.to_string()))
}

pub async fn execute_trades_on_chain(
    client: &SolanaClient,
    market_address: &str,
    trades: Vec<Trade>,
    share_type: ShareType,
) -> Result<String, Box<dyn std::error::Error>> {
    println!("  → Fetching market mints for {:?} shares...", share_type);
    let (collateral_mint, share_mint) = get_market_mints(
        &client.rpc_url(),
        &client.program_id(),
        market_address,
        share_type,
    )
    .await?;

    println!("  → Converting {} trades to match fills...", trades.len());
    let match_fills = convert_trades_to_match_fills(&trades, share_type)?;

    println!("  → Building transaction accounts...");
    let remaining_accounts = build_remaining_accounts(&trades, &collateral_mint, &share_mint)?;

    println!("  → Sending transaction to Solana...");
    let signature = client
        .execute_match_multi(market_address, match_fills, remaining_accounts)
        .await?;

    Ok(signature)
}

fn convert_trades_to_match_fills(
    trades: &[Trade],
    share_type: ShareType,
) -> Result<Vec<types::MatchFill>, Box<dyn std::error::Error>> {
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

        match_fills.push(types::MatchFill {
            shares,
            price,
            side: types::share_type_to_trade_side(share_type),
        });
    }

    Ok(match_fills)
}

fn build_remaining_accounts(
    trades: &[Trade],
    collateral_mint: &str,
    share_mint: &str,
) -> Result<Vec<types::AccountMeta>, Box<dyn std::error::Error>> {
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

        accounts.push(types::AccountMeta::new(buyer_collateral.to_string()));
        accounts.push(types::AccountMeta::new(seller_collateral.to_string()));
        accounts.push(types::AccountMeta::new(buyer_share.to_string()));
        accounts.push(types::AccountMeta::new(seller_share.to_string()));
        accounts.push(types::AccountMeta::new_readonly(buyer.to_string()));
        accounts.push(types::AccountMeta::new_readonly(seller.to_string()));
    }

    Ok(accounts)
}

pub async fn generate_split_transaction(
    client: &SolanaClient,
    market_address: &str,
    user_wallet: &str,
    amount: u64,
) -> Result<(String, String), Box<dyn std::error::Error>> {
    let rpc = RpcClient::new(client.rpc_url());
    let program_pubkey = Pubkey::from_str(client.program_id())?;
    let market_pubkey = Pubkey::from_str(market_address)?;
    let user_pubkey = Pubkey::from_str(user_wallet)?;

    let market = fetch_market(&rpc, &market_pubkey)?;

    let market_id = market.market_id;

    // Get fee payer
    let fee_payer = load_fee_payer()?;

    let recent_blockhash = rpc
        .get_latest_blockhash()
        .map_err(|e| anyhow::anyhow!("RPC error: {}", e))?;

    let collateral_mint = market.collateral_mint;
    let collateral_vault = market.collateral_vault;
    let yes_mint = market.yes_mint;
    let no_mint = market.no_mint;

    let user_collateral = get_ata_address(&user_pubkey, &collateral_mint);
    let yes_ata = get_ata_address(&user_pubkey, &yes_mint);
    let no_ata = get_ata_address(&user_pubkey, &no_mint);

    use anchor_client::Client;
    use std::sync::Arc;

    let cluster = detect_cluster(client.rpc_url());

    // Store fee_payer pubkey before moving into Arc
    let fee_payer_pubkey = fee_payer.pubkey();
    let fee_payer_arc: Arc<Keypair> = Arc::new(fee_payer);
    let provider = Client::new_with_options(
        cluster,
        fee_payer_arc.clone(),
        anchor_client::solana_sdk::commitment_config::CommitmentConfig::confirmed(),
    );

    let program = provider.program(program_pubkey)?;

    let instruction = program
        .request()
        .accounts(predix_program::accounts::SplitToken {
            market: market_pubkey,
            collateral_vault,
            yes_mint,
            no_mint,
            yes_ata,
            no_ata,
            user_collateral,
            token_program: spl_token::ID,
            system_program: solana_sdk::system_program::ID,
            associated_token_program: anchor_spl::associated_token::ID,
            rent: solana_sdk::sysvar::rent::ID,
            user: user_pubkey,
        })
        .args(predix_program::instruction::SplitToken { market_id, amount })
        .instructions()?
        .pop()
        .ok_or_else(|| anyhow::anyhow!("Failed to build instruction"))?;

    let message = Message::new(&[instruction], Some(&fee_payer_pubkey));
    let mut tx = Transaction::new_unsigned(message);

    tx.try_partial_sign(&[&*fee_payer_arc], recent_blockhash)
        .map_err(|e| anyhow::anyhow!("Failed to partially sign transaction: {}", e))?;

    let (tx_base64, recent_blockhash_str) = serialize_transaction(&tx, &recent_blockhash)?;

    Ok((tx_base64, recent_blockhash_str))
}

pub async fn generate_merge_transaction(
    client: &SolanaClient,
    market_address: &str,
    user_wallet: &str,
    amount: u64,
) -> Result<(String, String), Box<dyn std::error::Error>> {
    let rpc = RpcClient::new(client.rpc_url());
    let program_pubkey = Pubkey::from_str(client.program_id())?;
    let market_pubkey = Pubkey::from_str(market_address)?;
    let user_pubkey = Pubkey::from_str(user_wallet)?;

    let market = fetch_market(&rpc, &market_pubkey)?;
    let market_id = market.market_id;

    // Get fee payer
    let fee_payer = load_fee_payer()?;

    let recent_blockhash = rpc
        .get_latest_blockhash()
        .map_err(|e| anyhow::anyhow!("RPC error: {}", e))?;

    // Derive all required accounts
    let collateral_mint = market.collateral_mint;
    let collateral_vault = market.collateral_vault;
    let yes_mint = market.yes_mint;
    let no_mint = market.no_mint;

    println!("market {:?}", market);

    let user_collateral = get_ata_address(&user_pubkey, &collateral_mint);
    let yes_ata = get_ata_address(&user_pubkey, &yes_mint);
    let no_ata = get_ata_address(&user_pubkey, &no_mint);

    let cluster = detect_cluster(client.rpc_url());

    // Store fee_payer pubkey before moving into Arc
    let fee_payer_pubkey = fee_payer.pubkey();
    let fee_payer_arc: Arc<Keypair> = Arc::new(fee_payer);
    let provider = Client::new_with_options(
        cluster,
        fee_payer_arc.clone(),
        anchor_client::solana_sdk::commitment_config::CommitmentConfig::confirmed(),
    );

    let program = provider.program(program_pubkey)?;

    let instruction = program
        .request()
        .accounts(predix_program::accounts::MergeToken {
            user: user_pubkey,
            market: market_pubkey,
            collateral_vault,
            user_collateral,
            yes_mint,
            no_mint,
            yes_ata,
            no_ata,
            system_program: solana_sdk::system_program::ID,
            token_program: spl_token::ID,
        })
        .args(predix_program::instruction::MergeTokens { market_id, amount })
        .instructions()?
        .pop()
        .ok_or_else(|| anyhow::anyhow!("Failed to build instruction"))?;

    let message = Message::new(&[instruction], Some(&fee_payer_pubkey));
    let mut tx = Transaction::new_unsigned(message);

    tx.try_partial_sign(&[&*fee_payer_arc], recent_blockhash)
        .map_err(|e| anyhow::anyhow!("Failed to partially sign transaction: {}", e))?;

    let (tx_base64, recent_blockhash_str) = serialize_transaction(&tx, &recent_blockhash)?;

    Ok((tx_base64, recent_blockhash_str))
}
