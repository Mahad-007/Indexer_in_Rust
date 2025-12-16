//! BeanBee API Server
//!
//! REST API endpoints for the BeanBee frontend.

use std::{env, net::SocketAddr, sync::Arc};

use axum::{routing::get, Router};
use sqlx::{Pool, Postgres};
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod routes;

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    pub db_pool: Pool<Postgres>,
}

mod defaults {
    pub const API_PORT: &str = "8080";
    pub const API_HOST: &str = "0.0.0.0";
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "api=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting BeanBee API Server...");

    // Initialize database connection
    let db_pool = indexer_db::initialize_database().await?;
    tracing::info!("Connected to database");

    // Create app state
    let state = Arc::new(AppState { db_pool });

    // CORS configuration
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Build router
    let app = Router::new()
        // Root endpoint with API info
        .route("/", get(root))
        // Health check
        .route("/health", get(health_check))
        // API routes
        .nest("/api", routes::api_routes())
        // State and middleware
        .with_state(state)
        .layer(cors)
        .layer(tower_http::trace::TraceLayer::new_for_http());

    // Get port from environment
    let port = env::var("API_PORT")
        .unwrap_or_else(|_| defaults::API_PORT.to_string())
        .parse::<u16>()
        .unwrap_or(8080);

    let host = env::var("API_HOST").unwrap_or_else(|_| defaults::API_HOST.to_string());

    let addr: SocketAddr = format!("{}:{}", host, port).parse()?;
    tracing::info!("Listening on {}", addr);

    // Start server
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// Root endpoint - API information
async fn root() -> axum::response::Html<&'static str> {
    axum::response::Html(r#"
<!DOCTYPE html>
<html>
<head>
    <title>BeanBee API</title>
    <style>
        body { font-family: system-ui, sans-serif; max-width: 800px; margin: 50px auto; padding: 20px; background: #1a1a2e; color: #eee; }
        h1 { color: #f5a623; }
        a { color: #4fc3f7; }
        code { background: #333; padding: 2px 6px; border-radius: 4px; }
        .endpoint { margin: 10px 0; padding: 10px; background: #252540; border-radius: 8px; }
        .method { color: #4caf50; font-weight: bold; }
    </style>
</head>
<body>
    <h1>BeanBee API</h1>
    <p>BSC Memecoin Alpha Discovery Engine</p>

    <h2>Endpoints</h2>

    <div class="endpoint">
        <span class="method">GET</span> <a href="/health">/health</a> - Health check
    </div>

    <h3>Tokens</h3>
    <div class="endpoint">
        <span class="method">GET</span> <a href="/api/tokens/new">/api/tokens/new</a> - Newest tokens
    </div>
    <div class="endpoint">
        <span class="method">GET</span> <a href="/api/tokens/hot">/api/tokens/hot</a> - Hot tokens by volume
    </div>
    <div class="endpoint">
        <span class="method">GET</span> <code>/api/tokens/:address</code> - Token details
    </div>
    <div class="endpoint">
        <span class="method">GET</span> <code>/api/tokens/:address/swaps</code> - Token swaps
    </div>
    <div class="endpoint">
        <span class="method">GET</span> <code>/api/tokens/:address/holders</code> - Token holders
    </div>
    <div class="endpoint">
        <span class="method">GET</span> <code>/api/tokens/:address/chart</code> - Price chart data
    </div>

    <h3>Wallets</h3>
    <div class="endpoint">
        <span class="method">GET</span> <a href="/api/wallets">/api/wallets</a> - List tracked wallets
    </div>
    <div class="endpoint">
        <span class="method">POST</span> <code>/api/wallets</code> - Add wallet to track
    </div>
    <div class="endpoint">
        <span class="method">GET</span> <code>/api/wallets/:address</code> - Get wallet details
    </div>
    <div class="endpoint">
        <span class="method">DELETE</span> <code>/api/wallets/:address</code> - Remove wallet
    </div>
    <div class="endpoint">
        <span class="method">GET</span> <code>/api/wallets/:address/activity</code> - Wallet activity
    </div>

    <h3>Alerts</h3>
    <div class="endpoint">
        <span class="method">GET</span> <a href="/api/alerts/feed">/api/alerts/feed</a> - Alert feed
    </div>
</body>
</html>
    "#)
}

/// Health check endpoint
async fn health_check() -> &'static str {
    "OK"
}
