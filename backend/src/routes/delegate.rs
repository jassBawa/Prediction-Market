use std::{env, str::FromStr};

use axum::{extract::State, http::StatusCode, Extension, Json};
use base64;
use bincode;
use solana_sdk::{
    message::Message, pubkey::Pubkey, signature::Keypair, signer::Signer, transaction::Transaction,
};
use spl_token::instruction::approve_checked;

use crate::{
    auth::claims::AuthUser,
    models::delegate::{ApproveRequest, ApproveRes},
    state::Shared,
};

pub async fn delegate_approval(
    State(state): State<Shared>,
    Extension(user): Extension<AuthUser>,
    Json(payload): Json<ApproveRequest>,
) -> Result<Json<ApproveRes>, (StatusCode, String)> {
    let rpc_client = &state.rpc;
    let program_id = Pubkey::from_str(&payload.program_id).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            format!("Invalid program id: {}", e),
        )
    })?;

    let market_id = payload.market_id.parse::<u64>().map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            format!("Invalid market_id (must be u64): {}", e),
        )
    })?;

    // let payer_pub_key = env::var("FEE_PAYER_PUBLIC_KEY").map_err(|e| {
    //     (
    //         StatusCode::INTERNAL_SERVER_ERROR,
    //         format!("FEE_PAYER_PUBLIC_KEY not set: {}", e),
    //     )
    // })?;

    let payer_private_key = env::var("FEE_PAYER_PRIVATE_KEY").map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("FEE_PAYER_PRIVATE_KEY not set: {}", e),
        )
    })?;

    let fee_payer = Keypair::from_base58_string(&payer_private_key);
    let recent_blockhash = rpc_client.get_latest_blockhash().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("RPC error: {}", e),
        )
    })?;

    let wallet_pubkey = Pubkey::from_str(&user.solana_address).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            format!("Invalid wallet address: {}", e),
        )
    })?;
    let mint_pubkey = Pubkey::from_str(&payload.mint).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            format!("Invalid mint address: {}", e),
        )
    })?;
    let user_ata_pubkey = Pubkey::from_str(&payload.user_ata).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            format!("Invalid user ATA address: {}", e),
        )
    })?;

    let (market_pda, _bump) =
        Pubkey::find_program_address(&[b"market".as_ref(), &market_id.to_le_bytes()], &program_id);

    let approve_ix = approve_checked(
        &spl_token::id(),
        &user_ata_pubkey,
        &mint_pubkey,
        &market_pda,
        &wallet_pubkey,
        &[],
        payload.amount,
        payload.decimals,
    )
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to create approve instruction: {}", e),
        )
    })?;

    let message = Message::new(&[approve_ix], Some(&fee_payer.pubkey()));
    let mut tx = Transaction::new_unsigned(message);

    tx.try_partial_sign(&[fee_payer], recent_blockhash)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to partially sign transaction: {}", e),
            )
        })?;

    let serialized = bincode::serialize(&tx).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to serialize transaction: {}", e),
        )
    })?;

    #[allow(deprecated)]
    let tx_base64 = base64::encode(&serialized);
    Ok(Json(ApproveRes {
        tx_message: tx_base64,
        recent_blockhash: recent_blockhash.to_string(),
    }))
}
