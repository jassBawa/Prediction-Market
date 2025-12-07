use anchor_spl::associated_token::spl_associated_token_account::instruction::create_associated_token_account_idempotent;
use matching_engine::{ShareType, Side};
use rust_decimal::Decimal;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{instruction::Instruction, program_pack::Pack, pubkey::Pubkey};
use spl_token::instruction::approve_checked;

use std::str::FromStr;

use crate::solana::{
    accounts::get_ata_address,
    client::SolanaClient,
    market::{derive_market_pda, derive_share_mint, fetch_market},
    utils::{decimal_to_lamports, load_fee_payer, serialize_transaction},
};

use solana_sdk::{
    message::v0::Message as V0Message, message::VersionedMessage, signer::Signer,
    transaction::VersionedTransaction,
};
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

    let fee_payer = load_fee_payer()?;

    let recent_blockhash = rpc
        .get_latest_blockhash()
        .map_err(|e| anyhow::anyhow!("RPC error: {}", e))?;

    let (market_pda, _bump) = derive_market_pda(&program_pubkey, market_id);

    if market_pda != market_pubkey {
        return Err(anyhow::anyhow!(
            "Market PDA mismatch! Calculated: {}, Market address: {}. This suggests market_id {} doesn't match market address.",
            market_pda, market_pubkey, market_id
        ).into());
    }

    // Build instructions based on side
    let mut instructions: Vec<Instruction> = Vec::new();
    let (mint_pubkey, user_ata_pubkey, required_amount_lamports) = match side {
        Side::Bid => {
            let collateral_ata = get_ata_address(&user_pubkey, &collateral_mint);
            let required = decimal_to_lamports(qty * price)?;

            let share_mint = derive_share_mint(&program_pubkey, market_id, share_type);
            let create_ata_ix = create_associated_token_account_idempotent(
                &fee_payer.pubkey(),
                &user_pubkey,
                &share_mint,
                &spl_token::id(),
            );
            instructions.push(create_ata_ix);

            (collateral_mint, collateral_ata, required as u64)
        }
        Side::Ask => {
            let share_mint = derive_share_mint(&program_pubkey, market_id, share_type);
            let share_ata = get_ata_address(&user_pubkey, &share_mint);
            let required = decimal_to_lamports(qty)?;
            (share_mint, share_ata, required as u64)
        }
    };

    // Get actual decimals from the mint account
    let mint_account = rpc
        .get_account(&mint_pubkey)
        .map_err(|e| anyhow::anyhow!("Failed to fetch mint account: {}", e))?;
    let mint = spl_token::state::Mint::unpack(&mint_account.data)
        .map_err(|e| anyhow::anyhow!("Failed to unpack mint data: {}", e))?;
    let decimals = mint.decimals;

    // Check if token account exists
    let approve_ix = approve_checked(
        &spl_token::id(),
        &user_ata_pubkey,
        &mint_pubkey,
        &market_pda,
        &user_pubkey,
        &[],
        required_amount_lamports,
        decimals,
    )
    .map_err(|e| anyhow::anyhow!("Failed to create approve instruction: {}", e))?;

    if approve_ix.accounts.len() >= 4 {
        let delegate_in_instruction = approve_ix.accounts[2].pubkey;
        if delegate_in_instruction != market_pda {
            return Err(anyhow::anyhow!(
                "Approve instruction delegate {} doesn't match market_pda {}! This transaction would delegate to the wrong address!",
                delegate_in_instruction, market_pda
            ).into());
        }
    } else {
        return Err(anyhow::anyhow!(
            "Invalid approve_checked instruction: expected at least 4 accounts, got {}",
            approve_ix.accounts.len()
        )
        .into());
    }

    instructions.push(approve_ix);

    let v0_msg = V0Message::try_compile(&fee_payer.pubkey(), &instructions, &[], recent_blockhash)
        .map_err(|e| anyhow::anyhow!("Failed to compile V0 message: {}", e))?;

    let versioned_msg = VersionedMessage::V0(v0_msg);

    let num_signatures = versioned_msg.header().num_required_signatures as usize;
    let mut tx = VersionedTransaction {
        signatures: vec![solana_sdk::signature::Signature::default(); num_signatures],
        message: versioned_msg,
    };

    let message_bytes = tx.message.serialize();
    let fee_payer_signature = fee_payer
        .try_sign_message(&message_bytes)
        .map_err(|e| anyhow::anyhow!("Failed to sign with fee payer: {}", e))?;

    tx.signatures[0] = fee_payer_signature;

    let (tx_base64, recent_blockhash_str) = serialize_transaction(&tx, &recent_blockhash)?;

    Ok((tx_base64, recent_blockhash_str))
}
