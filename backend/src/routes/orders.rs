use std::str::FromStr;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Extension, Json,
};
use db::get_market_by_address;
use rust_decimal::Decimal;
use tokio::sync::{mpsc, oneshot};
use uuid::Uuid;

use crate::{
    auth::claims::AuthUser,
    models::order::{
        CancelReq, CancelRes, OrderBookResponse, OrderBookSide, PlaceOrderReq, PlaceOrderRes,
        SplitOrderReq,
    },
    state::Shared,
};
use matching_engine::{run_market_engine, EngineMsg, OrderEntry};

pub async fn place_order(
    State(state): State<Shared>,
    Extension(user): Extension<AuthUser>,
    Json(req): Json<PlaceOrderReq>,
) -> Result<Json<PlaceOrderRes>, (StatusCode, String)> {
    let price =
        Decimal::from_str(&req.price).map_err(|_| (StatusCode::BAD_REQUEST, "bad price".into()))?;
    let qty =
        Decimal::from_str(&req.qty).map_err(|_| (StatusCode::BAD_REQUEST, "bad qty".into()))?;

    println!("request body --> {:?}", req);

    let delegation_verified = crate::solana::verify_delegation(
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
        eprintln!("Delegation verification error: {}", e);
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
                format!("Failed to generate approval transaction: {}", e),
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

    println!(
        "Order received: {:?} {} {} for market {}",
        req.side, qty, price, req.market_id
    );
    let markets = state.markets.read().await;
    let tx = if let Some(tx) = markets.get(&req.market_id) {
        tx.clone()
    } else {
        drop(markets);

        let _market = get_market_by_address(&state.db, &req.market_address)
            .await
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "database error".into()))?
            .ok_or((StatusCode::NOT_FOUND, "market not found".into()))?;

        let mut markets = state.markets.write().await;

        // Double-check: another request may have created the engine
        if let Some(tx) = markets.get(&req.market_id) {
            tx.clone()
        } else {
            // create new market mpsc channel
            let (tx, rx) = mpsc::channel::<EngineMsg>(100);
            tokio::spawn(run_market_engine(rx));
            markets.insert(req.market_id.clone(), tx.clone());
            tx
        }
    };

    let (resp_tx, resp_rx) = oneshot::channel();
    tx.send(EngineMsg::PlaceOrder {
        side: req.side,
        share: req.share,
        order,
        resp: resp_tx,
    })
    .await
    .map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "engine send failed".into(),
        )
    })?;
    let (_id, trades, rem) = resp_rx
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "engine dropped".into()))?;

    if !trades.is_empty() {
        println!("Matched {} trade(s), remaining qty: {}", trades.len(), rem);
        println!("Executing on Solana...");
        let signature = crate::solana::execute_trades_on_chain(
            &state.rpc,
            &req.market_address,
            trades.clone(),
            req.share,
        )
        .await
        .map_err(|e| {
            eprintln!("Solana execution failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Solana error: {}", e),
            )
        })?;

        println!("Transaction confirmed: {}", signature);
    } else {
        println!(
            "Order placed in orderbook (no match), remaining qty: {}",
            rem
        );
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
    let markets = state.markets.read().await;
    let tx = if let Some(tx) = markets.get(&req.market_id) {
        tx.clone()
    } else {
        return Err((StatusCode::NOT_FOUND, "market not found".into()));
    };
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
    let markets = state.markets.read().await;
    let tx = if let Some(tx) = markets.get(&market_id) {
        tx.clone()
    } else {
        return Err((StatusCode::NOT_FOUND, "market not found".into()));
    };
    drop(markets);

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
