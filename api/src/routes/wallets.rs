//! Wallet API routes

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};

use indexer_db::entity::wallet_activity::WalletActivity;

use crate::AppState;

/// Wallet activity response item
#[derive(Debug, Serialize)]
pub struct WalletActivityItem {
    pub tx_hash: String,
    pub action: String,
    pub token_address: String,
    pub token_symbol: Option<String>,
    pub amount_tokens: Option<String>,
    pub amount_usd: Option<String>,
    pub timestamp: String,
}

impl From<WalletActivity> for WalletActivityItem {
    fn from(a: WalletActivity) -> Self {
        Self {
            tx_hash: a.tx_hash,
            action: a.action,
            token_address: a.token_address,
            token_symbol: a.token_symbol,
            amount_tokens: a.amount_tokens.map(|v| v.to_string()),
            amount_usd: a.amount_usd.map(|v| v.to_string()),
            timestamp: a.timestamp.to_rfc3339(),
        }
    }
}

/// Query params for activity endpoint
#[derive(Debug, Deserialize)]
pub struct ActivityParams {
    pub limit: Option<i32>,
}

/// GET /api/wallets/:address/activity
/// Returns recent activity for a wallet
pub async fn get_wallet_activity(
    State(state): State<Arc<AppState>>,
    Path(address): Path<String>,
    Query(params): Query<ActivityParams>,
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
