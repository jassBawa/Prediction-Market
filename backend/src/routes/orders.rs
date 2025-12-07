use axum::{
    extract::{Path, State},
    http::StatusCode,
    Extension, Json,
};

use tokio::sync::oneshot;
use uuid::Uuid;

use crate::{
    auth::claims::AuthUser,
    models::order::{
        CancelReq, CancelRes, MergeOrderReq, OrderBookResponse, OrderBookSide, PlaceOrderReq,
        PlaceOrderRes, SplitOrderReq,
    },
    routes::utils::{get_market_engine, get_or_create_market_engine, parse_decimal},
    state::Shared,
};
use db::get_market_by_address;
use matching_engine::{EngineMsg, OrderEntry};
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

pub async fn place_order(
    State(state): State<Shared>,
    Extension(user): Extension<AuthUser>,
    Json(req): Json<PlaceOrderReq>,
) -> Result<Json<PlaceOrderRes>, (StatusCode, String)> {
    let price = parse_decimal(&req.price, "price").map_err(|(code, msg)| (code, msg))?;
    let qty = parse_decimal(&req.qty, "qty").map_err(|(code, msg)| (code, msg))?;

    // Fetch market from DB FIRST (fast, no RPC)
    let market = get_market_by_address(&state.db, &req.market_address)
        .await
        .map_err(|e| {
            eprintln!("Database error fetching market: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Database error: {}", e),
            )
        })?
        .ok_or_else(|| {
            eprintln!("Market not found: {}", req.market_address);
            (StatusCode::NOT_FOUND, "Market not found".to_string())
        })?;

    // Parse market data for delegation check
    let market_id_u64 = market.market_id.parse::<u64>().map_err(|e| {
        eprintln!("Invalid market_id format: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Invalid market_id: {}", e),
        )
    })?;

    let collateral_mint = Pubkey::from_str(&market.collateral_mint).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Invalid collateral_mint: {}", e),
        )
    })?;

    let program_id = Pubkey::from_str(&market.program_id).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Invalid program_id: {}", e),
        )
    })?;

    let market_data = crate::solana::accounts::delegation::MarketData {
        market_id: market_id_u64,
        collateral_mint,
        program_id,
    };

    let delegation_verified = crate::solana::verify_delegation(
        &state.rpc,
        &req.market_address,
        &user.solana_address,
        req.side,
        req.share,
        price,
        qty,
        Some(market_data),
    )
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to verify delegation: {}", e),
        )
    })?;

    if !delegation_verified {
        let (tx_message, recent_blockhash) = crate::solana::generate_approval_transaction(
            &state.rpc,
            &req.market_address,
            &user.solana_address,
            req.side,
            req.share,
            price,
            qty,
        )
        .await
        .map_err(|e| {
            eprintln!("Failed to generate approval transaction: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Approval transaction error: {}", e),
            )
        })?;

        return Ok(Json(PlaceOrderRes::DelegationRequired {
            tx_message,
            recent_blockhash,
        }));
    }

    let order_id = Uuid::new_v4();

    let order = OrderEntry {
        id: order_id,
        user_id: user.solana_address.clone(),
        market_id: req.market_id.clone(),
        price,
        qty,
    };

    let tx = get_or_create_market_engine(&state, &req.market_id, &req.market_address)
        .await
        .map_err(|(code, msg)| {
            eprintln!("Market engine error: {}", msg);
            (code, msg)
        })?;

    let (resp_tx, resp_rx) = oneshot::channel();

    tx.send(EngineMsg::PlaceOrder {
        side: req.side,
        share: req.share,
        order,
        resp: resp_tx,
    })
    .await
    .map_err(|e| {
        eprintln!("Failed to send to matching engine: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Matching engine unavailable".into(),
        )
    })?;

    let (_id, trades, rem) = resp_rx.await.map_err(|e| {
        eprintln!("Matching engine error: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Matching engine error".into(),
        )
    })?;

    if !trades.is_empty() {
        // Use market data already fetched from DB above (no need to fetch again)
        let _signature = crate::solana::execute_trades_on_chain(
            &state.rpc,
            &req.market_address,
            trades.clone(),
            crate::solana::types::share_type_to_trade_side(req.share),
            market_id_u64,
            market.collateral_mint, // Use from DB instead of hardcoded value
        )
        .await
        .map_err(|e| {
            eprintln!("On-chain execution failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Transaction failed: {}", e),
            )
        })?;
    }
    Ok(Json(PlaceOrderRes::Success {
        order_id,
        trades,
        remaining_qty: rem,
    }))
}

pub async fn cancel_order(
    State(state): State<Shared>,
    Json(req): Json<CancelReq>,
) -> Result<Json<CancelRes>, (StatusCode, String)> {
    let tx = get_market_engine(&state, &req.market_id).await?;
    let (resp_tx, resp_rx) = oneshot::channel();

    tx.send(EngineMsg::CloseOrder {
        side: req.side,
        share: req.share,
        price: req.price,
        order_id: req.order_id,
        resp: resp_tx,
    })
    .await
    .map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "engine send failed".into(),
        )
    })?;
    let (res, message) = resp_rx
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "engine dropped".into()))?;
    if res {
        Ok(Json(CancelRes {
            success: res,
            message,
        }))
    } else {
        Err((StatusCode::INTERNAL_SERVER_ERROR, message))
    }
}

pub async fn get_orderbook(
    State(state): State<Shared>,
    Path(market_id): Path<String>,
) -> Result<Json<OrderBookResponse>, (StatusCode, String)> {
    let tx = get_market_engine(&state, &market_id).await?;

    let (resp_tx, resp_rx) = oneshot::channel();

    tx.send(EngineMsg::Snapshot { resp: resp_tx })
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "engine send failed".into(),
            )
        })?;

    let snapshot = resp_rx
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "engine dropped".into()))?;

    // snapshot structure: ((yes_bids, yes_asks), (no_bids, no_asks))
    let ((yes_bids, yes_asks), (no_bids, no_asks)) = snapshot;

    Ok(Json(OrderBookResponse {
        yes: OrderBookSide {
            bids: yes_bids,
            asks: yes_asks,
        },
        no: OrderBookSide {
            bids: no_bids,
            asks: no_asks,
        },
    }))
}

pub async fn split_order(
    State(state): State<Shared>,
    Extension(user): Extension<AuthUser>,
    Json(req): Json<SplitOrderReq>,
) -> Result<Json<PlaceOrderRes>, (StatusCode, String)> {
    let (tx_message, recent_blockhash) = crate::solana::generate_split_transaction(
        &state.rpc,
        &req.market_address,
        &user.solana_address,
        req.amount,
    )
    .await
    .map_err(|e| {
        eprintln!("Failed to generate split transaction: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to generate split transaction: {}", e),
        )
    })?;

    Ok(Json(PlaceOrderRes::DelegationRequired {
        tx_message,
        recent_blockhash,
    }))
}

pub async fn merge_order(
    State(state): State<Shared>,
    Extension(user): Extension<AuthUser>,
    Json(req): Json<MergeOrderReq>,
) -> Result<Json<PlaceOrderRes>, (StatusCode, String)> {
    let (tx_message, recent_blockhash) = crate::solana::generate_merge_transaction(
        &state.rpc,
        &req.market_address,
        &user.solana_address,
        req.amount,
    )
    .await
    .map_err(|e| {
        eprintln!("Failed to generate merge transaction: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to generate merge transaction: {}", e),
        )
    })?;

    Ok(Json(PlaceOrderRes::DelegationRequired {
        tx_message,
        recent_blockhash,
    }))
}
