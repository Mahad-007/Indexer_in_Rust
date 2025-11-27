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

/// Alert feed response item
#[derive(Debug, Serialize)]
pub struct AlertItem {
    pub id: i32,
    pub alert_type: String,
    pub token_address: Option<String>,
    pub token_symbol: Option<String>,
    pub wallet_address: Option<String>,
    pub title: String,
    pub message: Option<String>,
    pub bee_score: Option<i16>,
    pub amount_usd: Option<String>,
    pub change_percent: Option<String>,
    pub created_at: Option<String>,
}

impl From<AlertEvent> for AlertItem {
    fn from(a: AlertEvent) -> Self {
        Self {
            id: a.id,
            alert_type: a.alert_type,
            token_address: a.token_address,
            token_symbol: a.token_symbol,
            wallet_address: a.wallet_address,
            title: a.title,
            message: a.message,
            bee_score: a.bee_score,
            amount_usd: a.amount_usd.map(|v| v.to_string()),
            change_percent: a.change_percent.map(|v| v.to_string()),
            created_at: a.created_at.map(|dt| dt.to_rfc3339()),
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
