use std::env;

use sqlx::{
    postgres::{PgConnectOptions, PgPoolOptions},
    Pool, Postgres,
};

pub mod entity;

// Re-export commonly used types
pub use entity::{
    AlertEvent, EvmChains, EvmLogs, EvmSyncLogs, LpLock, Pair, PriceSnapshot, Swap, Token,
    TokenHolder, Wallet, WalletActivity, WalletWithStats,
};

mod defaults {
    pub const DATABASE_MAX_CONNECTIONS: &str = "5";
}

async fn create_pool(max_connections: u32) -> Result<Pool<Postgres>, sqlx::Error> {
    let conn = PgConnectOptions::new();

    PgPoolOptions::new()
        .max_connections(max_connections)
        .connect_with(conn)
        .await
}

pub async fn initialize_database() -> Result<Pool<Postgres>, sqlx::Error> {
    // Load .env file if present
    dotenvy::dotenv().ok();

    let db_max_connections = env::var("DATABASE_MAX_CONNECTIONS")
        .unwrap_or(String::from(defaults::DATABASE_MAX_CONNECTIONS))
        .parse::<u32>()
        .unwrap();

    let pool = create_pool(db_max_connections).await.unwrap();

    Ok(pool)
}
