
use sqlx::{
    types::{chrono, BigDecimal},
    Executor, Postgres,
};

/// Swap entity representing a DEX trade
#[derive(sqlx::FromRow, Debug, Clone)]
pub struct Swap {
    pub id: i32,
    pub tx_hash: String,
    pub block_number: i64,
    pub log_index: i32,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub pair_address: String,
    pub token_address: String,
    pub wallet_address: String,
    pub trade_type: String, // "buy" or "sell"
    pub amount_tokens: Option<BigDecimal>,
    pub amount_bnb: Option<BigDecimal>,
    pub amount_usd: Option<BigDecimal>,
    pub price_usd: Option<BigDecimal>,
    pub is_whale: Option<bool>,
}

/// Input for creating a new swap
#[derive(Debug, Clone)]
pub struct NewSwap {
    pub tx_hash: String,
    pub block_number: i64,
    pub log_index: i32,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub pair_address: String,
    pub token_address: String,
    pub wallet_address: String,
    pub trade_type: String,
    pub amount_tokens: Option<BigDecimal>,
    pub amount_bnb: Option<BigDecimal>,
    pub amount_usd: Option<BigDecimal>,
    pub price_usd: Option<BigDecimal>,
    pub is_whale: bool,
}

impl Swap {
    /// Create a new swap record
    pub async fn create<'c, E>(swap: &NewSwap, connection: E) -> Result<Swap, sqlx::Error>
    where
        E: Executor<'c, Database = Postgres>,
    {
        let query = r#"
            INSERT INTO swaps (
                tx_hash, block_number, log_index, timestamp, pair_address,
                token_address, wallet_address, trade_type, amount_tokens,
                amount_bnb, amount_usd, price_usd, is_whale
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            ON CONFLICT (tx_hash, log_index) DO NOTHING
            RETURNING *
        "#;

        sqlx::query_as::<_, Swap>(query)
            .bind(&swap.tx_hash)
            .bind(swap.block_number)
            .bind(swap.log_index)
            .bind(swap.timestamp)
            .bind(&swap.pair_address)
            .bind(&swap.token_address)
            .bind(&swap.wallet_address)
            .bind(&swap.trade_type)
            .bind(&swap.amount_tokens)
            .bind(&swap.amount_bnb)
            .bind(&swap.amount_usd)
            .bind(&swap.price_usd)
            .bind(swap.is_whale)
            .fetch_one(connection)
            .await
    }

    /// Find swaps by token address
    pub async fn find_by_token<'c, E>(
        token_address: &str,
        limit: i32,
        connection: E,
    ) -> Result<Vec<Swap>, sqlx::Error>
    where
        E: Executor<'c, Database = Postgres>,
    {
        sqlx::query_as::<_, Swap>(
            "SELECT * FROM swaps WHERE token_address = $1 ORDER BY timestamp DESC LIMIT $2",
        )
        .bind(token_address)
        .bind(limit)
        .fetch_all(connection)
        .await
    }

    /// Find swaps by wallet address
    pub async fn find_by_wallet<'c, E>(
        wallet_address: &str,
        limit: i32,
        connection: E,
    ) -> Result<Vec<Swap>, sqlx::Error>
    where
        E: Executor<'c, Database = Postgres>,
    {
        sqlx::query_as::<_, Swap>(
            "SELECT * FROM swaps WHERE wallet_address = $1 ORDER BY timestamp DESC LIMIT $2",
        )
        .bind(wallet_address)
        .bind(limit)
        .fetch_all(connection)
        .await
    }

    /// Find whale trades
    pub async fn find_whale_trades<'c, E>(
        limit: i32,
        connection: E,
    ) -> Result<Vec<Swap>, sqlx::Error>
    where
        E: Executor<'c, Database = Postgres>,
    {
        sqlx::query_as::<_, Swap>(
            "SELECT * FROM swaps WHERE is_whale = TRUE ORDER BY timestamp DESC LIMIT $1",
        )
        .bind(limit)
        .fetch_all(connection)
        .await
    }

    /// Get recent swaps for a token (for live feed)
    pub async fn find_recent_by_token<'c, E>(
        token_address: &str,
        since: chrono::DateTime<chrono::Utc>,
        connection: E,
    ) -> Result<Vec<Swap>, sqlx::Error>
    where
        E: Executor<'c, Database = Postgres>,
    {
        sqlx::query_as::<_, Swap>(
            "SELECT * FROM swaps WHERE token_address = $1 AND timestamp > $2 ORDER BY timestamp DESC",
        )
        .bind(token_address)
        .bind(since)
        .fetch_all(connection)
        .await
    }

    /// Count trades in last hour for a token
    pub async fn count_trades_1h<'c, E>(
        token_address: &str,
        connection: E,
    ) -> Result<(i64, i64, i64), sqlx::Error>
    where
        E: Executor<'c, Database = Postgres>,
    {
        let row: (i64, i64, i64) = sqlx::query_as(
            r#"
            SELECT
                COUNT(*) as total,
                COUNT(*) FILTER (WHERE trade_type = 'buy') as buys,
                COUNT(*) FILTER (WHERE trade_type = 'sell') as sells
            FROM swaps
            WHERE token_address = $1 AND timestamp > NOW() - INTERVAL '1 hour'
            "#,
        )
        .bind(token_address)
        .fetch_one(connection)
        .await?;

        Ok(row)
    }

    /// Calculate volume in last hour for a token
    pub async fn volume_1h<'c, E>(
        token_address: &str,
        connection: E,
    ) -> Result<BigDecimal, sqlx::Error>
    where
        E: Executor<'c, Database = Postgres>,
    {
        let volume: Option<BigDecimal> = sqlx::query_scalar(
            r#"
            SELECT COALESCE(SUM(amount_usd), 0)
            FROM swaps
            WHERE token_address = $1 AND timestamp > NOW() - INTERVAL '1 hour'
            "#,
        )
        .bind(token_address)
        .fetch_one(connection)
        .await?;

        Ok(volume.unwrap_or_else(|| BigDecimal::from(0)))
    }
}
