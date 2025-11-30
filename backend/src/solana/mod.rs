use anchor_client::solana_sdk::pubkey::Pubkey;
use matching_engine::{ShareType, Side, Trade};
use predix_program::state::Market;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{message::Message, signature::Keypair, signer::Signer, transaction::Transaction};
use spl_token::instruction::approve_checked;
use std::str::FromStr;
pub mod client;
use anyhow::{self, Result};
use base64;
use bincode;
pub mod types;

use client::SolanaClient;

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
    let market_pubkey = Pubkey::from_str(market_address)?;
    let user_pubkey = Pubkey::from_str(user_wallet)?;

    let account = rpc.get_account(&market_pubkey)?;
    use anchor_lang::AnchorDeserialize;
    let mut data = &account.data[8..];
    let market = Market::deserialize(&mut data)
        .map_err(|e| anyhow::anyhow!("Failed to deserialize market: {}", e))?;
    let market_id = market.market_id;
    let collateral_mint = market.collateral_mint;

    let (mint_pubkey, user_ata_pubkey, required_amount_lamports) = match side {
        Side::Bid => {
            let collateral_ata = get_ata_address(&user_pubkey, &collateral_mint);
            let required = (qty * price)
                .to_f64()
                .ok_or_else(|| anyhow::anyhow!("Invalid price/quantity: cannot convert to f64"))?
                * 1_000_000.0;
            (collateral_mint, collateral_ata, required as u64)
        }
        Side::Ask => {
            let (share_mint, _) = match share_type {
                ShareType::Yes => Pubkey::find_program_address(
                    &[b"yes_mint", &market_id.to_le_bytes()],
                    &program_pubkey,
                ),
                ShareType::No => Pubkey::find_program_address(
                    &[b"no_mint", &market_id.to_le_bytes()],
                    &program_pubkey,
                ),
            };
            let share_ata = get_ata_address(&user_pubkey, &share_mint);
            let required = qty
                .to_f64()
                .ok_or_else(|| anyhow::anyhow!("Invalid quantity: cannot convert to f64"))?
                * 1_000_000.0;
            (share_mint, share_ata, required as u64)
        }
    };

    // Get fee payer
    let payer_private_key = std::env::var("FEE_PAYER_PRIVATE_KEY")
        .map_err(|e| anyhow::anyhow!("FEE_PAYER_PRIVATE_KEY not set: {}", e))?;
    let fee_payer = Keypair::from_base58_string(&payer_private_key);

    let recent_blockhash = rpc
        .get_latest_blockhash()
        .map_err(|e| anyhow::anyhow!("RPC error: {}", e))?;

    println!(
        "Recent blockhash generated at: {:?}",
        std::time::SystemTime::now()
    );
    println!("Blockhash: {} (expires in ~60 seconds)", recent_blockhash);

    let (market_pda, _bump) = Pubkey::find_program_address(
        &[b"market".as_ref(), &market_id.to_le_bytes()],
        &program_pubkey,
    );

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

    let message = Message::new(&[approve_ix], Some(&fee_payer.pubkey()));
    let mut tx = Transaction::new_unsigned(message);

    // Partially sign with fee payer
    tx.try_partial_sign(&[&fee_payer], recent_blockhash)
        .map_err(|e| anyhow::anyhow!("Failed to partially sign transaction: {}", e))?;

    // Serialize transaction
    let serialized = bincode::serialize(&tx)
        .map_err(|e| anyhow::anyhow!("Failed to serialize transaction: {}", e))?;

    #[allow(deprecated)]
    let tx_base64 = base64::encode(&serialized);
    let recent_blockhash_str = recent_blockhash.to_string();

    println!("Transaction generated successfully!");
    println!("   Transaction size: {} bytes", serialized.len());
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

    let account = rpc.get_account(&market_pubkey)?;

    use anchor_lang::AnchorDeserialize;

    let mut data = &account.data[8..];
    let market = Market::deserialize(&mut data)
        .map_err(|e| anyhow::anyhow!("Failed to deserialize market: {}", e))?;

    let market_id = market.market_id;
    let collateral_mint = market.collateral_mint;

    let (yes_mint, _) =
        Pubkey::find_program_address(&[b"yes_mint", &market_id.to_le_bytes()], &program_pubkey);

    let (no_mint, _) =
        Pubkey::find_program_address(&[b"no_mint", &market_id.to_le_bytes()], &program_pubkey);

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

fn decimal_to_lamports(decimal: Decimal) -> Result<u64, Box<dyn std::error::Error>> {
    let lamports = (decimal.to_f64().unwrap() * 1_000_000.0) as u64;
    Ok(lamports)
}

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
    println!("{market_address} {user_wallet} {}", client.program_id());
    let program_pubkey = Pubkey::from_str(client.program_id())?;
    let market_pubkey = Pubkey::from_str(market_address)?;
    let user_pubkey = Pubkey::from_str(user_wallet)?;

    let account = rpc.get_account(&market_pubkey)?;
    use anchor_lang::AnchorDeserialize;
    let mut data = &account.data[8..];
    let market = Market::deserialize(&mut data)
        .map_err(|e| anyhow::anyhow!("Failed to deserialize market: {}", e))?;

    let market_id = market.market_id;
    let collateral_mint = market.collateral_mint;

    // Get the bump from the market account (it's stored in the account data)
    // The market account has a bump field - we should use it for verification
    // But first, let's calculate the PDA to compare
    let (calculated_pda, calculated_bump) =
        Pubkey::find_program_address(&[b"market", &market_id.to_le_bytes()], &program_pubkey);

    // Verify the market_pubkey matches the calculated PDA (it should, since market is a PDA)
    if market_pubkey != calculated_pda {
        return Err(anyhow::anyhow!(
            "Market address {} does not match calculated PDA {} for market_id {}",
            market_pubkey,
            calculated_pda,
            market_id
        )
        .into());
    }

    let market_pda = calculated_pda;
    let _bump = calculated_bump;

    println!("=== DELEGATION VERIFICATION DEBUG ===");
    println!("Market address: {}", market_address);
    println!("Market ID: {}", market_id);
    println!("Program ID: {}", program_pubkey);
    println!("Calculated market_pda: {}", market_pda);
    println!("Market PDA bump: {}", _bump);

    let (token_account_pubkey, required_amount_lamports) = match side {
        Side::Bid => {
            let collateral_ata = get_ata_address(&user_pubkey, &collateral_mint);
            let required = (qty * price).to_f64().unwrap() * 1_000_000.0;
            (collateral_ata, required as u64)
        }
        Side::Ask => {
            let (share_mint, _) = match share_type {
                ShareType::Yes => Pubkey::find_program_address(
                    &[b"yes_mint", &market_id.to_le_bytes()],
                    &program_pubkey,
                ),
                ShareType::No => Pubkey::find_program_address(
                    &[b"no_mint", &market_id.to_le_bytes()],
                    &program_pubkey,
                ),
            };
            let share_ata = get_ata_address(&user_pubkey, &share_mint);
            let required = qty.to_f64().unwrap() * 1_000_000.0;
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

    println!("User pubkey: {}", user_pubkey);
    println!("Collateral mint: {}", collateral_mint);
    println!("ATA: {}", token_account_pubkey);
    println!("Required amount: {}", required_amount_lamports);
    println!("Market ID: {}", market_id);
    println!("Calculated market_pda: {}", market_pda);
    println!(
        "Delegate in token account: {:?}",
        token_account_info.delegate
    );
    println!("Delegated amount: {}", token_account_info.delegated_amount);

    match token_account_info.delegate {
        Some(delegate) if delegate == market_pda => {
            println!("✅ Delegate matches market_pda! Checking amounts...");
            let sufficient = token_account_info.delegated_amount >= required_amount_lamports;
            println!(
                "Delegated: {}, Required: {}, Sufficient: {}",
                token_account_info.delegated_amount, required_amount_lamports, sufficient
            );
            Ok(sufficient)
        }
        Some(delegate) => {
            println!("❌ DELEGATE MISMATCH!");
            println!("   Expected delegate (market_pda): {}", market_pda);
            println!("   Found delegate in token account: {}", delegate);
            println!("   Token account ATA: {}", token_account_pubkey);
            println!("   User wallet: {}", user_pubkey);
            println!("   Market address: {}", market_pubkey);
            println!("   Market ID: {}", market_id);
            println!();
            println!("    CRITICAL: You have delegated to a DIFFERENT address!");
            println!(
                "   This means you signed a transaction that delegated to: {}",
                delegate
            );
            println!("   But you need to delegate to: {}", market_pda);
            println!();
            println!("   SOLUTION:");
            println!("   1. Make sure you are signing the FRESH transaction from this API call");
            println!("   2. Do NOT use any cached/old transactions");
            println!(
                "   3. The transaction we just generated will delegate to: {}",
                market_pda
            );
            println!("   4. Sign and submit the NEW transaction IMMEDIATELY (< 60 seconds)");
            println!("   5. Wait for on-chain confirmation");
            println!("   6. Retry placing the order");
            println!();
            println!("     NOTE: The approve_checked instruction will REPLACE the old delegate.");
            println!("   Just sign the NEW transaction we returned - don't revoke manually.");
            Ok(false)
        }
        None => {
            println!("❌ No delegate set in token account");
            Ok(false)
        }
    }
}

fn get_ata_address(owner: &Pubkey, mint: &Pubkey) -> Pubkey {
    let associated_token_program_id =
        Pubkey::from_str("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL")
            .expect("Invalid Associated Token Program ID");
    let (ata, _) = Pubkey::find_program_address(
        &[owner.as_ref(), spl_token::id().as_ref(), mint.as_ref()],
        &associated_token_program_id,
    );
    ata
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

    // Get market account to extract market_id
    let account = rpc.get_account(&market_pubkey)?;
    use anchor_lang::AnchorDeserialize;
    let mut data = &account.data[8..];
    let market = Market::deserialize(&mut data)
        .map_err(|e| anyhow::anyhow!("Failed to deserialize market: {}", e))?;
    let market_id = market.market_id;

    // Get fee payer
    let payer_private_key = std::env::var("FEE_PAYER_PRIVATE_KEY")
        .map_err(|e| anyhow::anyhow!("FEE_PAYER_PRIVATE_KEY not set: {}", e))?;
    let fee_payer = Keypair::from_base58_string(&payer_private_key);

    let recent_blockhash = rpc
        .get_latest_blockhash()
        .map_err(|e| anyhow::anyhow!("RPC error: {}", e))?;

    println!("=== GENERATING SPLIT TOKEN TRANSACTION ===");
    println!("Market address: {}", market_address);
    println!("Market ID: {}", market_id);
    println!("User wallet: {}", user_wallet);
    println!("Amount: {}", amount);

    // Derive all required accounts
    let collateral_mint = market.collateral_mint;
    let collateral_vault = market.collateral_vault;
    let yes_mint = market.yes_mint;
    let no_mint = market.no_mint;

    println!("market {:?}", market);

    let user_collateral = get_ata_address(&user_pubkey, &collateral_mint);
    let yes_ata = get_ata_address(&user_pubkey, &yes_mint);
    let no_ata = get_ata_address(&user_pubkey, &no_mint);

    println!("User collateral ATA: {}", user_collateral);
    println!("Yes ATA: {}", yes_ata);
    println!("No ATA: {}", no_ata);

    // Build the split_token instruction using anchor_client
    use anchor_client::{Client, Cluster};
    use std::sync::Arc;

    let cluster =
        if client.rpc_url().contains("localhost") || client.rpc_url().contains("127.0.0.1") {
            Cluster::Localnet
        } else if client.rpc_url().contains("devnet") {
            Cluster::Devnet
        } else if client.rpc_url().contains("mainnet") {
            Cluster::Mainnet
        } else {
            Cluster::Custom(client.rpc_url().to_string(), client.rpc_url().to_string())
        };

    // Store fee_payer pubkey before moving into Arc
    let fee_payer_pubkey = fee_payer.pubkey();
    let fee_payer_arc: Arc<Keypair> = Arc::new(fee_payer);
    let provider = Client::new_with_options(
        cluster,
        fee_payer_arc.clone(),
        anchor_client::solana_sdk::commitment_config::CommitmentConfig::confirmed(),
    );

    let program = provider.program(program_pubkey)?;

    // Build the instruction using anchor_client pattern (like pm-cli)
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

    // Create transaction with fee payer as payer, user will sign as required signer
    let message = Message::new(&[instruction], Some(&fee_payer_pubkey));
    let mut tx = Transaction::new_unsigned(message);

    // Partially sign with fee payer
    tx.try_partial_sign(&[&*fee_payer_arc], recent_blockhash)
        .map_err(|e| anyhow::anyhow!("Failed to partially sign transaction: {}", e))?;

    // Serialize transaction
    let serialized = bincode::serialize(&tx)
        .map_err(|e| anyhow::anyhow!("Failed to serialize transaction: {}", e))?;

    #[allow(deprecated)]
    let tx_base64 = base64::encode(&serialized);
    let recent_blockhash_str = recent_blockhash.to_string();

    println!("Transaction generated successfully!");
    println!("   Transaction size: {} bytes", serialized.len());
    println!("   Base64 length: {} chars", tx_base64.len());
    println!("   Recent blockhash: {}", recent_blockhash_str);
    println!();

    Ok((tx_base64, recent_blockhash_str))
}
