//! Alert API routes

use std::sync::Arc;

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};

use indexer_db::entity::alert::AlertEvent;

use crate::AppState;

/// Helper to convert BigDecimal to f64
fn bd_to_f64(bd: &sqlx::types::BigDecimal) -> f64 {
    bd.to_string().parse().unwrap_or(0.0)
}

/// Map backend alert types to frontend types
fn map_alert_type(alert_type: &str) -> &str {
    match alert_type {
        "new_token" => "token_signal",
        "whale_buy" | "whale_sell" => "wallet_activity",
        "price_pump" | "price_dump" => "token_signal",
        "lp_locked" | "lp_unlocking" => "token_signal",
        "high_bee_score" => "token_signal",
        "dev_sell" => "wallet_activity",
        "filter_match" => "filter_match",
        _ => "token_signal",
    }
}

/// Alert feed response item - matches frontend Alert interface
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AlertItem {
    pub id: String,
    #[serde(rename = "type")]
    pub alert_type: String,
    pub title: String,
    pub message: String,
    pub token_address: Option<String>,
    pub wallet_address: Option<String>,
    pub timestamp: String,
    pub is_read: bool,
    // Additional fields for enrichment
    pub bee_score: Option<i16>,
    pub amount_usd: Option<f64>,
    pub change_percent: Option<f64>,
}

impl From<AlertEvent> for AlertItem {
    fn from(a: AlertEvent) -> Self {
        Self {
            id: a.id.to_string(),
            alert_type: map_alert_type(&a.alert_type).to_string(),
            title: a.title,
            message: a.message.unwrap_or_default(),
            token_address: a.token_address,
            wallet_address: a.wallet_address,
            timestamp: a.created_at.map(|dt| dt.to_rfc3339()).unwrap_or_default(),
            is_read: false, // Default to unread - frontend manages read state locally
            bee_score: a.bee_score,
            amount_usd: a.amount_usd.as_ref().map(bd_to_f64),
            change_percent: a.change_percent.as_ref().map(bd_to_f64),
        }
    }
}

/// Query params for feed endpoint
#[derive(Debug, Deserialize)]
pub struct FeedParams {
    pub limit: Option<i32>,
    pub alert_type: Option<String>,
}

/// GET /api/alerts/feed
/// Returns recent alerts for the live feed
pub async fn get_alert_feed(
    State(state): State<Arc<AppState>>,
    Query(params): Query<FeedParams>,
) -> impl IntoResponse {
    let limit = params.limit.unwrap_or(50).min(200);

    let result = if let Some(alert_type) = params.alert_type {
        AlertEvent::find_by_type(&alert_type, limit, &state.db_pool).await
    } else {
        AlertEvent::find_recent(limit, &state.db_pool).await
    };

    match result {
        Ok(alerts) => {
            let items: Vec<AlertItem> = alerts.into_iter().map(Into::into).collect();
            Json(items).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to get alert feed: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response()
        }
    }
}
