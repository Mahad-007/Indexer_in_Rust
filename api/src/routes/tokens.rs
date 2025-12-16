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

/// Helper to convert BigDecimal to f64
fn bd_to_f64(bd: &sqlx::types::BigDecimal) -> f64 {
    bd.to_string().parse().unwrap_or(0.0)
}

/// Token list response item - matches frontend Token interface
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenListItem {
    pub address: String,
    pub name: String,
    pub symbol: String,
    pub price: f64,
    pub price_change1h: f64,
    pub price_change24h: f64,
    pub liquidity: f64,
    pub market_cap: f64,
    pub volume1h: f64,
    pub volume24h: f64,
    pub holders: i32,
    pub bee_score: i16,
    pub safety_score: i16,
    pub traction_score: i16,
    pub lp_locked: bool,
    pub dev_holdings: f64,
    pub sniper_ratio: f64,
    pub created_at: String,
    pub chain: String,
}

impl From<Token> for TokenListItem {
    fn from(t: Token) -> Self {
        Self {
            address: t.address,
            name: t.name.unwrap_or_else(|| "Unknown".to_string()),
            symbol: t.symbol.unwrap_or_else(|| "???".to_string()),
            price: t.price_usd.as_ref().map(bd_to_f64).unwrap_or(0.0),
            price_change1h: t.price_change_1h.as_ref().map(bd_to_f64).unwrap_or(0.0),
            price_change24h: t.price_change_24h.as_ref().map(bd_to_f64).unwrap_or(0.0),
            liquidity: t.liquidity_usd.as_ref().map(bd_to_f64).unwrap_or(0.0),
            market_cap: t.market_cap_usd.as_ref().map(bd_to_f64).unwrap_or(0.0),
            volume1h: t.volume_1h_usd.as_ref().map(bd_to_f64).unwrap_or(0.0),
            volume24h: t.volume_24h_usd.as_ref().map(bd_to_f64).unwrap_or(0.0),
            holders: t.holder_count.unwrap_or(0),
            bee_score: t.bee_score.unwrap_or(0),
            safety_score: t.safety_score.unwrap_or(0),
            traction_score: t.traction_score.unwrap_or(0),
            lp_locked: t.lp_locked.unwrap_or(false),
            dev_holdings: t.dev_holdings_percent.as_ref().map(bd_to_f64).unwrap_or(0.0),
            sniper_ratio: t.sniper_ratio.as_ref().map(bd_to_f64).unwrap_or(0.0),
            created_at: t.created_at.map(|dt| dt.to_rfc3339()).unwrap_or_else(|| Utc::now().to_rfc3339()),
            chain: "BSC".to_string(),
        }
    }
}

/// Token detail response - extended version for single token view
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenDetail {
    pub address: String,
    pub name: String,
    pub symbol: String,
    pub decimals: i16,
    pub pair_address: Option<String>,
    pub creator_address: Option<String>,
    pub created_at: String,
    pub block_number: Option<i64>,

    // Price metrics
    pub price: f64,
    pub price_bnb: f64,
    pub price_change1h: f64,
    pub price_change24h: f64,
    pub market_cap: f64,
    pub liquidity: f64,
    pub liquidity_bnb: f64,
    pub volume1h: f64,
    pub volume24h: f64,

    // Trading metrics
    pub trades1h: i32,
    pub trades24h: i32,
    pub buys1h: i32,
    pub sells1h: i32,

    // Holder metrics
    pub holders: i32,
    pub top10_holder_percent: f64,
    pub dev_holdings: f64,
    pub sniper_ratio: f64,

    // Safety
    pub lp_locked: bool,
    pub lp_lock_percent: f64,
    pub lp_unlock_date: Option<String>,
    pub ownership_renounced: bool,

    // BeeScore
    pub bee_score: i16,
    pub safety_score: i16,
    pub traction_score: i16,

    pub chain: String,
    pub last_updated: Option<String>,
}

impl From<Token> for TokenDetail {
    fn from(t: Token) -> Self {
        Self {
            address: t.address,
            name: t.name.unwrap_or_else(|| "Unknown".to_string()),
            symbol: t.symbol.unwrap_or_else(|| "???".to_string()),
            decimals: t.decimals.unwrap_or(18),
            pair_address: t.pair_address,
            creator_address: t.creator_address,
            created_at: t.created_at.map(|dt| dt.to_rfc3339()).unwrap_or_else(|| Utc::now().to_rfc3339()),
            block_number: t.block_number,

            price: t.price_usd.as_ref().map(bd_to_f64).unwrap_or(0.0),
            price_bnb: t.price_bnb.as_ref().map(bd_to_f64).unwrap_or(0.0),
            price_change1h: t.price_change_1h.as_ref().map(bd_to_f64).unwrap_or(0.0),
            price_change24h: t.price_change_24h.as_ref().map(bd_to_f64).unwrap_or(0.0),
            market_cap: t.market_cap_usd.as_ref().map(bd_to_f64).unwrap_or(0.0),
            liquidity: t.liquidity_usd.as_ref().map(bd_to_f64).unwrap_or(0.0),
            liquidity_bnb: t.liquidity_bnb.as_ref().map(bd_to_f64).unwrap_or(0.0),
            volume1h: t.volume_1h_usd.as_ref().map(bd_to_f64).unwrap_or(0.0),
            volume24h: t.volume_24h_usd.as_ref().map(bd_to_f64).unwrap_or(0.0),

            trades1h: t.trades_1h.unwrap_or(0),
            trades24h: t.trades_24h.unwrap_or(0),
            buys1h: t.buys_1h.unwrap_or(0),
            sells1h: t.sells_1h.unwrap_or(0),

            holders: t.holder_count.unwrap_or(0),
            top10_holder_percent: t.top_10_holder_percent.as_ref().map(bd_to_f64).unwrap_or(0.0),
            dev_holdings: t.dev_holdings_percent.as_ref().map(bd_to_f64).unwrap_or(0.0),
            sniper_ratio: t.sniper_ratio.as_ref().map(bd_to_f64).unwrap_or(0.0),

            lp_locked: t.lp_locked.unwrap_or(false),
            lp_lock_percent: t.lp_lock_percent.as_ref().map(bd_to_f64).unwrap_or(0.0),
            lp_unlock_date: t.lp_unlock_date.map(|dt| dt.to_rfc3339()),
            ownership_renounced: t.ownership_renounced.unwrap_or(false),

            bee_score: t.bee_score.unwrap_or(0),
            safety_score: t.safety_score.unwrap_or(0),
            traction_score: t.traction_score.unwrap_or(0),

            chain: "BSC".to_string(),
            last_updated: t.last_updated.map(|dt| dt.to_rfc3339()),
        }
    }
}

/// Swap response item
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SwapItem {
    pub tx_hash: String,
    pub wallet_address: String,
    pub trade_type: String,
    pub amount_tokens: f64,
    pub amount_usd: f64,
    pub price_usd: f64,
    pub is_whale: bool,
    pub timestamp: String,
}

impl From<Swap> for SwapItem {
    fn from(s: Swap) -> Self {
        Self {
            tx_hash: s.tx_hash,
            wallet_address: s.wallet_address,
            trade_type: s.trade_type,
            amount_tokens: s.amount_tokens.as_ref().map(bd_to_f64).unwrap_or(0.0),
            amount_usd: s.amount_usd.as_ref().map(bd_to_f64).unwrap_or(0.0),
            price_usd: s.price_usd.as_ref().map(bd_to_f64).unwrap_or(0.0),
            is_whale: s.is_whale.unwrap_or(false),
            timestamp: s.timestamp.to_rfc3339(),
        }
    }
}

/// Holder response item
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HolderItem {
    pub wallet_address: String,
    pub balance: f64,
    pub percent_of_supply: f64,
    pub is_dev: bool,
    pub is_sniper: bool,
}

impl From<TokenHolder> for HolderItem {
    fn from(h: TokenHolder) -> Self {
        Self {
            wallet_address: h.wallet_address,
            balance: h.balance.as_ref().map(bd_to_f64).unwrap_or(0.0),
            percent_of_supply: h.percent_of_supply.as_ref().map(bd_to_f64).unwrap_or(0.0),
            is_dev: h.is_dev.unwrap_or(false),
            is_sniper: h.is_sniper.unwrap_or(false),
        }
    }
}

/// Chart data point
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChartDataPoint {
    pub timestamp: String,
    pub price_usd: f64,
    pub liquidity_usd: f64,
    pub volume_usd: f64,
}

impl From<PriceSnapshot> for ChartDataPoint {
    fn from(s: PriceSnapshot) -> Self {
        Self {
            timestamp: s.timestamp.to_rfc3339(),
            price_usd: s.price_usd.as_ref().map(bd_to_f64).unwrap_or(0.0),
            liquidity_usd: s.liquidity_usd.as_ref().map(bd_to_f64).unwrap_or(0.0),
            volume_usd: s.volume_usd.as_ref().map(bd_to_f64).unwrap_or(0.0),
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
