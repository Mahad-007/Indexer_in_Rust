
use sqlx::{
    types::{chrono, BigDecimal},
    Executor, Postgres,
};

/// PriceSnapshot entity for historical price charts
#[derive(sqlx::FromRow, Debug, Clone)]
pub struct PriceSnapshot {
    pub id: i32,
    pub token_address: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub price_usd: Option<BigDecimal>,
    pub price_bnb: Option<BigDecimal>,
    pub liquidity_usd: Option<BigDecimal>,
    pub volume_usd: Option<BigDecimal>,
    pub market_cap_usd: Option<BigDecimal>,
    pub holder_count: Option<i32>,
}

/// Input for creating a new price snapshot
#[derive(Debug, Clone)]
pub struct NewPriceSnapshot {
    pub token_address: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub price_usd: Option<BigDecimal>,
    pub price_bnb: Option<BigDecimal>,
    pub liquidity_usd: Option<BigDecimal>,
    pub volume_usd: Option<BigDecimal>,
    pub market_cap_usd: Option<BigDecimal>,
    pub holder_count: Option<i32>,
}

impl PriceSnapshot {
    /// Create a new price snapshot
    pub async fn create<'c, E>(
        snapshot: &NewPriceSnapshot,
        connection: E,
    ) -> Result<PriceSnapshot, sqlx::Error>
    where
        E: Executor<'c, Database = Postgres>,
    {
        let query = r#"
            INSERT INTO price_snapshots (
                token_address, timestamp, price_usd, price_bnb,
                liquidity_usd, volume_usd, market_cap_usd, holder_count
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (token_address, timestamp) DO UPDATE SET
                price_usd = EXCLUDED.price_usd,
                price_bnb = EXCLUDED.price_bnb,
                liquidity_usd = EXCLUDED.liquidity_usd,
                volume_usd = EXCLUDED.volume_usd,
                market_cap_usd = EXCLUDED.market_cap_usd,
                holder_count = EXCLUDED.holder_count
            RETURNING *
        "#;

        sqlx::query_as::<_, PriceSnapshot>(query)
            .bind(&snapshot.token_address)
            .bind(snapshot.timestamp)
            .bind(&snapshot.price_usd)
            .bind(&snapshot.price_bnb)
            .bind(&snapshot.liquidity_usd)
            .bind(&snapshot.volume_usd)
            .bind(&snapshot.market_cap_usd)
            .bind(snapshot.holder_count)
            .fetch_one(connection)
            .await
    }

    /// Get price history for a token
    pub async fn find_by_token<'c, E>(
        token_address: &str,
        limit: i32,
        connection: E,
    ) -> Result<Vec<PriceSnapshot>, sqlx::Error>
    where
        E: Executor<'c, Database = Postgres>,
    {
        sqlx::query_as::<_, PriceSnapshot>(
            "SELECT * FROM price_snapshots WHERE token_address = $1 ORDER BY timestamp DESC LIMIT $2",
        )
        .bind(token_address)
        .bind(limit)
        .fetch_all(connection)
        .await
    }

    /// Get price history within a time range
    pub async fn find_in_range<'c, E>(
        token_address: &str,
        start: chrono::DateTime<chrono::Utc>,
        end: chrono::DateTime<chrono::Utc>,
        connection: E,
    ) -> Result<Vec<PriceSnapshot>, sqlx::Error>
    where
        E: Executor<'c, Database = Postgres>,
    {
        sqlx::query_as::<_, PriceSnapshot>(
            r#"
            SELECT * FROM price_snapshots
            WHERE token_address = $1 AND timestamp >= $2 AND timestamp <= $3
            ORDER BY timestamp ASC
            "#,
        )
        .bind(token_address)
        .bind(start)
        .bind(end)
        .fetch_all(connection)
        .await
    }

    /// Get latest snapshot for a token
    pub async fn find_latest<'c, E>(
        token_address: &str,
        connection: E,
    ) -> Result<Option<PriceSnapshot>, sqlx::Error>
    where
        E: Executor<'c, Database = Postgres>,
    {
        sqlx::query_as::<_, PriceSnapshot>(
            "SELECT * FROM price_snapshots WHERE token_address = $1 ORDER BY timestamp DESC LIMIT 1",
        )
        .bind(token_address)
        .fetch_optional(connection)
        .await
    }

    /// Get 1 hour ago snapshot for price change calculation
    pub async fn find_1h_ago<'c, E>(
        token_address: &str,
        connection: E,
    ) -> Result<Option<PriceSnapshot>, sqlx::Error>
    where
        E: Executor<'c, Database = Postgres>,
    {
        sqlx::query_as::<_, PriceSnapshot>(
            r#"
            SELECT * FROM price_snapshots
            WHERE token_address = $1 AND timestamp <= NOW() - INTERVAL '1 hour'
            ORDER BY timestamp DESC
            LIMIT 1
            "#,
        )
        .bind(token_address)
        .fetch_optional(connection)
        .await
    }

    /// Delete old snapshots (for cleanup)
    pub async fn delete_old<'c, E>(
        older_than_days: i32,
        connection: E,
    ) -> Result<u64, sqlx::Error>
    where
        E: Executor<'c, Database = Postgres>,
    {
        let result = sqlx::query(
            "DELETE FROM price_snapshots WHERE timestamp < NOW() - ($1 || ' days')::INTERVAL",
        )
        .bind(older_than_days)
        .execute(connection)
        .await?;

        Ok(result.rows_affected())
    }
}
