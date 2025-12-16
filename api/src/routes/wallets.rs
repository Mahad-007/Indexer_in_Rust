//! Wallet API routes

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};

use indexer_db::entity::{
    wallet::{NewWallet, Wallet, WalletWithStats},
    wallet_activity::WalletActivity,
};

use crate::AppState;

/// Helper to convert BigDecimal to f64
fn bd_to_f64(bd: &sqlx::types::BigDecimal) -> f64 {
    bd.to_string().parse().unwrap_or(0.0)
}

/// Wallet list response item - matches frontend Wallet interface
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WalletItem {
    pub address: String,
    pub label: Option<String>,
    pub token_count: i64,
    pub estimated_value: f64,
    pub last_activity: Option<String>,
}

impl From<WalletWithStats> for WalletItem {
    fn from(w: WalletWithStats) -> Self {
        Self {
            address: w.address,
            label: w.label,
            token_count: w.token_count,
            estimated_value: w.estimated_value_usd.as_ref().map(bd_to_f64).unwrap_or(0.0),
            last_activity: w.last_activity.map(|dt| dt.to_rfc3339()),
        }
    }
}

impl From<Wallet> for WalletItem {
    fn from(w: Wallet) -> Self {
        Self {
            address: w.address,
            label: w.label,
            token_count: w.token_count.unwrap_or(0) as i64,
            estimated_value: w.estimated_value_usd.as_ref().map(bd_to_f64).unwrap_or(0.0),
            last_activity: w.last_activity.map(|dt| dt.to_rfc3339()),
        }
    }
}

/// Wallet activity response item - matches frontend WalletActivity interface
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WalletActivityItem {
    pub id: String,
    pub wallet_address: String,
    pub action: String,
    pub token_address: String,
    pub token_symbol: String,
    pub amount: f64,
    pub value: f64,
    pub timestamp: String,
}

impl From<WalletActivity> for WalletActivityItem {
    fn from(a: WalletActivity) -> Self {
        Self {
            id: a.id.to_string(),
            wallet_address: a.wallet_address,
            action: a.action,
            token_address: a.token_address,
            token_symbol: a.token_symbol.unwrap_or_else(|| "???".to_string()),
            amount: a.amount_tokens.as_ref().map(bd_to_f64).unwrap_or(0.0),
            value: a.amount_usd.as_ref().map(bd_to_f64).unwrap_or(0.0),
            timestamp: a.timestamp.to_rfc3339(),
        }
    }
}

/// Query params for list endpoints
#[derive(Debug, Deserialize)]
pub struct ListParams {
    pub limit: Option<i32>,
}

/// Request body for creating a wallet
#[derive(Debug, Deserialize)]
pub struct CreateWalletRequest {
    pub address: String,
    pub label: Option<String>,
}

/// GET /api/wallets
/// Returns list of all tracked wallets with computed stats
pub async fn get_wallets(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ListParams>,
) -> impl IntoResponse {
    let limit = params.limit.unwrap_or(50).min(100);

    match Wallet::find_all_with_stats(limit, &state.db_pool).await {
        Ok(wallets) => {
            let items: Vec<WalletItem> = wallets.into_iter().map(Into::into).collect();
            Json(items).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to get wallets: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response()
        }
    }
}

/// POST /api/wallets
/// Add a new wallet to track
pub async fn create_wallet(
    State(state): State<Arc<AppState>>,
    Json(body): Json<CreateWalletRequest>,
) -> impl IntoResponse {
    let new_wallet = NewWallet {
        address: body.address.to_lowercase(),
        label: body.label,
    };

    match Wallet::create(&new_wallet, &state.db_pool).await {
        Ok(wallet) => {
            let item: WalletItem = wallet.into();
            (StatusCode::CREATED, Json(item)).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to create wallet: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response()
        }
    }
}

/// GET /api/wallets/:address
/// Get a specific wallet
pub async fn get_wallet(
    State(state): State<Arc<AppState>>,
    Path(address): Path<String>,
) -> impl IntoResponse {
    match Wallet::find_by_address(&address, &state.db_pool).await {
        Ok(Some(wallet)) => {
            let item: WalletItem = wallet.into();
            Json(item).into_response()
        }
        Ok(None) => (StatusCode::NOT_FOUND, "Wallet not found").into_response(),
        Err(e) => {
            tracing::error!("Failed to get wallet: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response()
        }
    }
}

/// DELETE /api/wallets/:address
/// Remove a wallet from tracking
pub async fn delete_wallet(
    State(state): State<Arc<AppState>>,
    Path(address): Path<String>,
) -> impl IntoResponse {
    match Wallet::delete_by_address(&address, &state.db_pool).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => (StatusCode::NOT_FOUND, "Wallet not found").into_response(),
        Err(e) => {
            tracing::error!("Failed to delete wallet: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response()
        }
    }
}

/// GET /api/wallets/:address/activity
/// Returns recent activity for a wallet
pub async fn get_wallet_activity(
    State(state): State<Arc<AppState>>,
    Path(address): Path<String>,
    Query(params): Query<ListParams>,
) -> impl IntoResponse {
    let limit = params.limit.unwrap_or(50).min(500);

    match WalletActivity::find_by_wallet(&address, limit, &state.db_pool).await {
        Ok(activities) => {
            let items: Vec<WalletActivityItem> = activities.into_iter().map(Into::into).collect();
            Json(items).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to get wallet activity: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response()
        }
    }
}
