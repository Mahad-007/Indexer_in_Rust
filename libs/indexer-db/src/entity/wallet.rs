use sqlx::{
    types::{chrono, BigDecimal},
    Executor, Postgres,
};

/// Wallet entity for tracking wallets with labels and computed stats
#[derive(sqlx::FromRow, Debug, Clone)]
pub struct Wallet {
    pub id: i32,
    pub address: String,
    pub label: Option<String>,
    pub token_count: Option<i32>,
    pub estimated_value_usd: Option<BigDecimal>,
    pub last_activity: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Input for creating a new wallet
#[derive(Debug, Clone)]
pub struct NewWallet {
    pub address: String,
    pub label: Option<String>,
}

/// Wallet with computed statistics from wallet_activity
#[derive(sqlx::FromRow, Debug, Clone)]
pub struct WalletWithStats {
    pub address: String,
    pub label: Option<String>,
    pub token_count: i64,
    pub estimated_value_usd: Option<BigDecimal>,
    pub last_activity: Option<chrono::DateTime<chrono::Utc>>,
}

impl Wallet {
    /// Create a new wallet record
    pub async fn create<'c, E>(wallet: &NewWallet, connection: E) -> Result<Wallet, sqlx::Error>
    where
        E: Executor<'c, Database = Postgres>,
    {
        let query = r#"
            INSERT INTO wallets (address, label)
            VALUES ($1, $2)
            ON CONFLICT (address) DO UPDATE SET
                label = COALESCE(EXCLUDED.label, wallets.label),
                updated_at = NOW()
            RETURNING *
        "#;

        sqlx::query_as::<_, Wallet>(query)
            .bind(&wallet.address)
            .bind(&wallet.label)
            .fetch_one(connection)
            .await
    }

    /// Find wallet by address
    pub async fn find_by_address<'c, E>(
        address: &str,
        connection: E,
    ) -> Result<Option<Wallet>, sqlx::Error>
    where
        E: Executor<'c, Database = Postgres>,
    {
        sqlx::query_as::<_, Wallet>("SELECT * FROM wallets WHERE address = $1")
            .bind(address)
            .fetch_optional(connection)
            .await
    }

    /// Get all wallets ordered by estimated value
    pub async fn find_all<'c, E>(
        limit: i32,
        connection: E,
    ) -> Result<Vec<Wallet>, sqlx::Error>
    where
        E: Executor<'c, Database = Postgres>,
    {
        sqlx::query_as::<_, Wallet>(
            "SELECT * FROM wallets ORDER BY estimated_value_usd DESC NULLS LAST, created_at DESC LIMIT $1",
        )
        .bind(limit)
        .fetch_all(connection)
        .await
    }

    /// Get all wallets with computed stats from wallet_activity
    pub async fn find_all_with_stats<'c, E>(
        limit: i32,
        connection: E,
    ) -> Result<Vec<WalletWithStats>, sqlx::Error>
    where
        E: Executor<'c, Database = Postgres>,
    {
        let query = r#"
            SELECT 
                w.address,
                w.label,
                COALESCE(stats.token_count, 0) as token_count,
                COALESCE(stats.total_value, w.estimated_value_usd) as estimated_value_usd,
                COALESCE(stats.last_activity, w.last_activity) as last_activity
            FROM wallets w
            LEFT JOIN (
                SELECT 
                    wallet_address,
                    COUNT(DISTINCT token_address) as token_count,
                    SUM(CASE WHEN action = 'buy' THEN amount_usd ELSE -amount_usd END) as total_value,
                    MAX(timestamp) as last_activity
                FROM wallet_activity
                GROUP BY wallet_address
            ) stats ON w.address = stats.wallet_address
            ORDER BY estimated_value_usd DESC NULLS LAST, w.created_at DESC
            LIMIT $1
        "#;

        sqlx::query_as::<_, WalletWithStats>(query)
            .bind(limit)
            .fetch_all(connection)
            .await
    }

    /// Delete wallet by address
    pub async fn delete_by_address<'c, E>(
        address: &str,
        connection: E,
    ) -> Result<bool, sqlx::Error>
    where
        E: Executor<'c, Database = Postgres>,
    {
        let result = sqlx::query("DELETE FROM wallets WHERE address = $1")
            .bind(address)
            .execute(connection)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Update wallet label
    pub async fn update_label<'c, E>(
        address: &str,
        label: Option<&str>,
        connection: E,
    ) -> Result<Option<Wallet>, sqlx::Error>
    where
        E: Executor<'c, Database = Postgres>,
    {
        sqlx::query_as::<_, Wallet>(
            "UPDATE wallets SET label = $2, updated_at = NOW() WHERE address = $1 RETURNING *",
        )
        .bind(address)
        .bind(label)
        .fetch_optional(connection)
        .await
    }

    /// Update wallet computed stats (can be called periodically)
    pub async fn update_stats<'c, E>(
        address: &str,
        token_count: i32,
        estimated_value: &BigDecimal,
        last_activity: Option<chrono::DateTime<chrono::Utc>>,
        connection: E,
    ) -> Result<(), sqlx::Error>
    where
        E: Executor<'c, Database = Postgres>,
    {
        sqlx::query(
            r#"
            UPDATE wallets SET
                token_count = $2,
                estimated_value_usd = $3,
                last_activity = $4,
                updated_at = NOW()
            WHERE address = $1
            "#,
        )
        .bind(address)
        .bind(token_count)
        .bind(estimated_value)
        .bind(last_activity)
        .execute(connection)
        .await?;

        Ok(())
    }

    /// Count total wallets
    pub async fn count<'c, E>(connection: E) -> Result<i64, sqlx::Error>
    where
        E: Executor<'c, Database = Postgres>,
    {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM wallets")
            .fetch_one(connection)
            .await?;

        Ok(count)
    }
}

