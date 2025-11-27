//! Token API routes

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};

use indexer_db::entity::{
    price_snapshot::PriceSnapshot,
    swap::Swap,
    token::Token,
    token_holder::TokenHolder,
};

use crate::AppState;

/// Token list response item
#[derive(Debug, Serialize)]
pub struct TokenListItem {
    pub address: String,
    pub name: Option<String>,
    pub symbol: Option<String>,
    pub price_usd: Option<String>,
    pub price_change_1h: Option<String>,
    pub liquidity_usd: Option<String>,
    pub volume_1h_usd: Option<String>,
    pub volume_24h_usd: Option<String>,
    pub trades_1h: Option<i32>,
    pub holder_count: Option<i32>,
    pub bee_score: Option<i16>,
    pub created_at: Option<String>,
}

impl From<Token> for TokenListItem {
    fn from(t: Token) -> Self {
        Self {
            address: t.address,
            name: t.name,
            symbol: t.symbol,
            price_usd: t.price_usd.map(|v| v.to_string()),
            price_change_1h: t.price_change_1h.map(|v| v.to_string()),
            liquidity_usd: t.liquidity_usd.map(|v| v.to_string()),
            volume_1h_usd: t.volume_1h_usd.map(|v| v.to_string()),
            volume_24h_usd: t.volume_24h_usd.map(|v| v.to_string()),
            trades_1h: t.trades_1h,
            holder_count: t.holder_count,
            bee_score: t.bee_score,
            created_at: t.created_at.map(|dt| dt.to_rfc3339()),
        }
    }
}

/// Token detail response
#[derive(Debug, Serialize)]
pub struct TokenDetail {
    pub address: String,
    pub name: Option<String>,
    pub symbol: Option<String>,
    pub decimals: Option<i16>,
    pub pair_address: Option<String>,
    pub creator_address: Option<String>,
    pub created_at: Option<String>,
    pub block_number: Option<i64>,

    // Price metrics
    pub price_usd: Option<String>,
    pub price_bnb: Option<String>,
    pub price_change_1h: Option<String>,
    pub price_change_24h: Option<String>,
    pub market_cap_usd: Option<String>,
    pub liquidity_usd: Option<String>,
    pub liquidity_bnb: Option<String>,
    pub volume_1h_usd: Option<String>,
    pub volume_24h_usd: Option<String>,

    // Trading metrics
    pub trades_1h: Option<i32>,
    pub trades_24h: Option<i32>,
    pub buys_1h: Option<i32>,
    pub sells_1h: Option<i32>,

    // Holder metrics
    pub holder_count: Option<i32>,
    pub top_10_holder_percent: Option<String>,
    pub dev_holdings_percent: Option<String>,
    pub sniper_ratio: Option<String>,

    // Safety
    pub lp_locked: Option<bool>,
    pub lp_lock_percent: Option<String>,
    pub lp_unlock_date: Option<String>,
    pub ownership_renounced: Option<bool>,

    // BeeScore
    pub bee_score: Option<i16>,
    pub safety_score: Option<i16>,
    pub traction_score: Option<i16>,

    pub last_updated: Option<String>,
}

impl From<Token> for TokenDetail {
    fn from(t: Token) -> Self {
        Self {
            address: t.address,
            name: t.name,
            symbol: t.symbol,
            decimals: t.decimals,
            pair_address: t.pair_address,
            creator_address: t.creator_address,
            created_at: t.created_at.map(|dt| dt.to_rfc3339()),
            block_number: t.block_number,

            price_usd: t.price_usd.map(|v| v.to_string()),
            price_bnb: t.price_bnb.map(|v| v.to_string()),
            price_change_1h: t.price_change_1h.map(|v| v.to_string()),
            price_change_24h: t.price_change_24h.map(|v| v.to_string()),
            market_cap_usd: t.market_cap_usd.map(|v| v.to_string()),
            liquidity_usd: t.liquidity_usd.map(|v| v.to_string()),
            liquidity_bnb: t.liquidity_bnb.map(|v| v.to_string()),
            volume_1h_usd: t.volume_1h_usd.map(|v| v.to_string()),
            volume_24h_usd: t.volume_24h_usd.map(|v| v.to_string()),

            trades_1h: t.trades_1h,
            trades_24h: t.trades_24h,
            buys_1h: t.buys_1h,
            sells_1h: t.sells_1h,

            holder_count: t.holder_count,
            top_10_holder_percent: t.top_10_holder_percent.map(|v| v.to_string()),
            dev_holdings_percent: t.dev_holdings_percent.map(|v| v.to_string()),
            sniper_ratio: t.sniper_ratio.map(|v| v.to_string()),

            lp_locked: t.lp_locked,
            lp_lock_percent: t.lp_lock_percent.map(|v| v.to_string()),
            lp_unlock_date: t.lp_unlock_date.map(|dt| dt.to_rfc3339()),
            ownership_renounced: t.ownership_renounced,

            bee_score: t.bee_score,
            safety_score: t.safety_score,
            traction_score: t.traction_score,

            last_updated: t.last_updated.map(|dt| dt.to_rfc3339()),
        }
    }
}

/// Swap response item
#[derive(Debug, Serialize)]
pub struct SwapItem {
    pub tx_hash: String,
    pub wallet_address: String,
    pub trade_type: String,
    pub amount_tokens: Option<String>,
    pub amount_usd: Option<String>,
    pub price_usd: Option<String>,
    pub is_whale: Option<bool>,
    pub timestamp: String,
}

impl From<Swap> for SwapItem {
    fn from(s: Swap) -> Self {
        Self {
            tx_hash: s.tx_hash,
            wallet_address: s.wallet_address,
            trade_type: s.trade_type,
            amount_tokens: s.amount_tokens.map(|v| v.to_string()),
            amount_usd: s.amount_usd.map(|v| v.to_string()),
            price_usd: s.price_usd.map(|v| v.to_string()),
            is_whale: s.is_whale,
            timestamp: s.timestamp.to_rfc3339(),
        }
    }
}

/// Holder response item
#[derive(Debug, Serialize)]
pub struct HolderItem {
    pub wallet_address: String,
    pub balance: Option<String>,
    pub percent_of_supply: Option<String>,
    pub is_dev: Option<bool>,
    pub is_sniper: Option<bool>,
}

impl From<TokenHolder> for HolderItem {
    fn from(h: TokenHolder) -> Self {
        Self {
            wallet_address: h.wallet_address,
            balance: h.balance.map(|v| v.to_string()),
            percent_of_supply: h.percent_of_supply.map(|v| v.to_string()),
            is_dev: h.is_dev,
            is_sniper: h.is_sniper,
        }
    }
}

/// Chart data point
#[derive(Debug, Serialize)]
pub struct ChartDataPoint {
    pub timestamp: String,
    pub price_usd: Option<String>,
    pub liquidity_usd: Option<String>,
    pub volume_usd: Option<String>,
}

impl From<PriceSnapshot> for ChartDataPoint {
    fn from(s: PriceSnapshot) -> Self {
        Self {
            timestamp: s.timestamp.to_rfc3339(),
            price_usd: s.price_usd.map(|v| v.to_string()),
            liquidity_usd: s.liquidity_usd.map(|v| v.to_string()),
            volume_usd: s.volume_usd.map(|v| v.to_string()),
        }
    }
}

/// Query params for list endpoints
#[derive(Debug, Deserialize)]
pub struct ListParams {
    pub limit: Option<i32>,
}

/// Query params for chart endpoint
#[derive(Debug, Deserialize)]
pub struct ChartParams {
    pub interval: Option<String>, // "5m", "1h"
    pub range: Option<String>,    // "1h", "6h", "24h"
}

/// GET /api/tokens/new
/// Returns newest tokens sorted by created_at
pub async fn get_new_tokens(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ListParams>,
) -> impl IntoResponse {
    let limit = params.limit.unwrap_or(50).min(100);

    match Token::find_newest(limit, &state.db_pool).await {
        Ok(tokens) => {
            let items: Vec<TokenListItem> = tokens.into_iter().map(Into::into).collect();
            Json(items).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to get new tokens: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response()
        }
    }
}

/// GET /api/tokens/hot
/// Returns hot tokens sorted by volume + BeeScore
pub async fn get_hot_tokens(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ListParams>,
) -> impl IntoResponse {
    let limit = params.limit.unwrap_or(50).min(100);

    match Token::find_hot(limit, &state.db_pool).await {
        Ok(tokens) => {
            let items: Vec<TokenListItem> = tokens.into_iter().map(Into::into).collect();
            Json(items).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to get hot tokens: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response()
        }
    }
}

/// GET /api/tokens/:address
/// Returns full token details
pub async fn get_token(
    State(state): State<Arc<AppState>>,
    Path(address): Path<String>,
) -> impl IntoResponse {
    match Token::find_by_address(&address, &state.db_pool).await {
        Ok(Some(token)) => Json(TokenDetail::from(token)).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "Token not found").into_response(),
        Err(e) => {
            tracing::error!("Failed to get token: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response()
        }
    }
}

/// GET /api/tokens/:address/swaps
/// Returns recent swaps for a token
pub async fn get_token_swaps(
    State(state): State<Arc<AppState>>,
    Path(address): Path<String>,
    Query(params): Query<ListParams>,
) -> impl IntoResponse {
    let limit = params.limit.unwrap_or(100).min(500);

    match Swap::find_by_token(&address, limit, &state.db_pool).await {
        Ok(swaps) => {
            let items: Vec<SwapItem> = swaps.into_iter().map(Into::into).collect();
            Json(items).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to get token swaps: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response()
        }
    }
}

/// GET /api/tokens/:address/holders
/// Returns top holders for a token
pub async fn get_token_holders(
    State(state): State<Arc<AppState>>,
    Path(address): Path<String>,
    Query(params): Query<ListParams>,
) -> impl IntoResponse {
    let limit = params.limit.unwrap_or(20).min(100);

    match TokenHolder::find_top_holders(&address, limit, &state.db_pool).await {
        Ok(holders) => {
            let items: Vec<HolderItem> = holders.into_iter().map(Into::into).collect();
            Json(items).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to get token holders: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response()
        }
    }
}

/// GET /api/tokens/:address/chart
/// Returns price snapshots for charting
pub async fn get_token_chart(
    State(state): State<Arc<AppState>>,
    Path(address): Path<String>,
    Query(params): Query<ChartParams>,
) -> impl IntoResponse {
    let range = params.range.unwrap_or_else(|| "24h".to_string());

    let hours = match range.as_str() {
        "1h" => 1,
        "6h" => 6,
        "24h" => 24,
        "7d" => 168,
        _ => 24,
    };

    let start = Utc::now() - Duration::hours(hours);
    let end = Utc::now();

    match PriceSnapshot::find_in_range(&address, start, end, &state.db_pool).await {
        Ok(snapshots) => {
            let items: Vec<ChartDataPoint> = snapshots.into_iter().map(Into::into).collect();
            Json(items).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to get chart data: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response()
        }
    }
}
