//! API route definitions

pub mod alerts;
pub mod tokens;
pub mod wallets;

use std::sync::Arc;

use axum::{routing::get, Router};

use crate::AppState;

/// Create all API routes
pub fn api_routes() -> Router<Arc<AppState>> {
    Router::new()
        // Token routes
        .route("/tokens/new", get(tokens::get_new_tokens))
        .route("/tokens/hot", get(tokens::get_hot_tokens))
        .route("/tokens/:address", get(tokens::get_token))
        .route("/tokens/:address/swaps", get(tokens::get_token_swaps))
        .route("/tokens/:address/holders", get(tokens::get_token_holders))
        .route("/tokens/:address/chart", get(tokens::get_token_chart))
        // Wallet routes
        .route("/wallets/:address/activity", get(wallets::get_wallet_activity))
        // Alert routes
        .route("/alerts/feed", get(alerts::get_alert_feed))
}
