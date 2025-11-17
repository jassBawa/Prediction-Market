use std::str::FromStr;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use db::get_market_by_address;
use rust_decimal::Decimal;
use serde::Serialize;
use tokio::sync::{mpsc, oneshot};
use uuid::Uuid;

use crate::{
    models::order::{CancelReq, CancelRes, PlaceOrderReq, PlaceOrderRes},
    state::Shared,
};
use matching_engine::{run_market_engine, EngineMsg, OrderEntry, SnapshotData};

pub async fn place_order(
    State(state): State<Shared>,
    Json(req): Json<PlaceOrderReq>,
) -> Result<Json<PlaceOrderRes>, (StatusCode, String)> {
    let price =
        Decimal::from_str(&req.price).map_err(|_| (StatusCode::BAD_REQUEST, "bad price".into()))?;
    let qty =
        Decimal::from_str(&req.qty).map_err(|_| (StatusCode::BAD_REQUEST, "bad qty".into()))?;

    let order_id = Uuid::new_v4();
    let order = OrderEntry {
        id: order_id,
        user_id: req.user_id.clone(),
        market_id: req.market_id.clone(),
        price,
        qty,
    };

    let markets = state.markets.read().await;
    // retrieve market
    let tx = if let Some(tx) = markets.get(&req.market_id) {
        tx.clone()
    } else {
        // drop(markets);

        let _market = get_market_by_address(&state.db, &req.market_id)
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

    Ok(Json(PlaceOrderRes {
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

/// Response structure for orderbook snapshot
#[derive(Debug, Serialize)]
pub struct OrderBookResponse {
    pub yes: OrderBookSide,
    pub no: OrderBookSide,
}

#[derive(Debug, Serialize)]
pub struct OrderBookSide {
    pub bids: Vec<SnapshotData>,
    pub asks: Vec<SnapshotData>,
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
