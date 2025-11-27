
use sqlx::{
    types::{chrono, BigDecimal},
    Executor, Postgres,
};

/// WalletActivity entity for tracking wallet transactions
#[derive(sqlx::FromRow, Debug, Clone)]
pub struct WalletActivity {
    pub id: i32,
    pub wallet_address: String,
    pub tx_hash: String,
    pub block_number: i64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub action: String, // "buy", "sell", "transfer_in", "transfer_out"
    pub token_address: String,
    pub token_symbol: Option<String>,
    pub amount_tokens: Option<BigDecimal>,
    pub amount_usd: Option<BigDecimal>,
}

/// Input for creating new wallet activity
#[derive(Debug, Clone)]
pub struct NewWalletActivity {
    pub wallet_address: String,
    pub tx_hash: String,
    pub block_number: i64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub action: String,
    pub token_address: String,
    pub token_symbol: Option<String>,
    pub amount_tokens: Option<BigDecimal>,
    pub amount_usd: Option<BigDecimal>,
}

impl WalletActivity {
    /// Create a new wallet activity record
    pub async fn create<'c, E>(
        activity: &NewWalletActivity,
        connection: E,
    ) -> Result<WalletActivity, sqlx::Error>
    where
        E: Executor<'c, Database = Postgres>,
    {
        let query = r#"
            INSERT INTO wallet_activity (
                wallet_address, tx_hash, block_number, timestamp,
                action, token_address, token_symbol, amount_tokens, amount_usd
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            ON CONFLICT (tx_hash, wallet_address, token_address, action) DO NOTHING
            RETURNING *
        "#;

        sqlx::query_as::<_, WalletActivity>(query)
            .bind(&activity.wallet_address)
            .bind(&activity.tx_hash)
            .bind(activity.block_number)
            .bind(activity.timestamp)
            .bind(&activity.action)
            .bind(&activity.token_address)
            .bind(&activity.token_symbol)
            .bind(&activity.amount_tokens)
            .bind(&activity.amount_usd)
            .fetch_one(connection)
            .await
    }

    /// Get activity for a wallet
    pub async fn find_by_wallet<'c, E>(
        wallet_address: &str,
        limit: i32,
        connection: E,
    ) -> Result<Vec<WalletActivity>, sqlx::Error>
    where
        E: Executor<'c, Database = Postgres>,
    {
        sqlx::query_as::<_, WalletActivity>(
            "SELECT * FROM wallet_activity WHERE wallet_address = $1 ORDER BY timestamp DESC LIMIT $2",
        )
        .bind(wallet_address)
        .bind(limit)
        .fetch_all(connection)
        .await
    }

    /// Get activity for a token
    pub async fn find_by_token<'c, E>(
        token_address: &str,
        limit: i32,
        connection: E,
    ) -> Result<Vec<WalletActivity>, sqlx::Error>
    where
        E: Executor<'c, Database = Postgres>,
    {
        sqlx::query_as::<_, WalletActivity>(
            "SELECT * FROM wallet_activity WHERE token_address = $1 ORDER BY timestamp DESC LIMIT $2",
        )
        .bind(token_address)
        .bind(limit)
        .fetch_all(connection)
        .await
    }

    /// Get recent activity for a wallet on a specific token
    pub async fn find_by_wallet_and_token<'c, E>(
        wallet_address: &str,
        token_address: &str,
        limit: i32,
        connection: E,
    ) -> Result<Vec<WalletActivity>, sqlx::Error>
    where
        E: Executor<'c, Database = Postgres>,
    {
        sqlx::query_as::<_, WalletActivity>(
            r#"
            SELECT * FROM wallet_activity
            WHERE wallet_address = $1 AND token_address = $2
            ORDER BY timestamp DESC
            LIMIT $3
            "#,
        )
        .bind(wallet_address)
        .bind(token_address)
        .bind(limit)
        .fetch_all(connection)
        .await
    }

    /// Count unique tokens a wallet has interacted with
    pub async fn count_unique_tokens<'c, E>(
        wallet_address: &str,
        connection: E,
    ) -> Result<i64, sqlx::Error>
    where
        E: Executor<'c, Database = Postgres>,
    {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(DISTINCT token_address) FROM wallet_activity WHERE wallet_address = $1",
        )
        .bind(wallet_address)
        .fetch_one(connection)
        .await?;

        Ok(count)
    }

    /// Get wallet's profit/loss summary for a token
    pub async fn calculate_pnl<'c, E>(
        wallet_address: &str,
        token_address: &str,
        connection: E,
    ) -> Result<(BigDecimal, BigDecimal), sqlx::Error>
    where
        E: Executor<'c, Database = Postgres>,
    {
        let row: (Option<BigDecimal>, Option<BigDecimal>) = sqlx::query_as(
            r#"
            SELECT
                COALESCE(SUM(CASE WHEN action = 'sell' THEN amount_usd ELSE 0 END), 0) as sold,
                COALESCE(SUM(CASE WHEN action = 'buy' THEN amount_usd ELSE 0 END), 0) as bought
            FROM wallet_activity
            WHERE wallet_address = $1 AND token_address = $2
            "#,
        )
        .bind(wallet_address)
        .bind(token_address)
        .fetch_one(connection)
        .await?;

        Ok((
            row.0.unwrap_or_else(|| BigDecimal::from(0)),
            row.1.unwrap_or_else(|| BigDecimal::from(0)),
        ))
    }
}
