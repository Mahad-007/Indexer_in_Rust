
use sqlx::{
    types::{chrono, BigDecimal},
    Executor, Postgres,
};

/// LpLock entity representing a liquidity lock
#[derive(sqlx::FromRow, Debug, Clone)]
pub struct LpLock {
    pub id: i32,
    pub token_address: String,
    pub pair_address: String,
    pub lock_contract: String,
    pub lock_contract_name: Option<String>,
    pub locked_amount: Option<BigDecimal>,
    pub locked_percent: Option<BigDecimal>,
    pub lock_date: Option<chrono::DateTime<chrono::Utc>>,
    pub unlock_date: Option<chrono::DateTime<chrono::Utc>>,
    pub tx_hash: Option<String>,
    pub block_number: Option<i64>,
    pub is_active: Option<bool>,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Input for creating a new LP lock
#[derive(Debug, Clone)]
pub struct NewLpLock {
    pub token_address: String,
    pub pair_address: String,
    pub lock_contract: String,
    pub lock_contract_name: String,
    pub locked_amount: BigDecimal,
    pub locked_percent: BigDecimal,
    pub lock_date: chrono::DateTime<chrono::Utc>,
    pub unlock_date: chrono::DateTime<chrono::Utc>,
    pub tx_hash: String,
    pub block_number: i64,
}

impl LpLock {
    /// Create a new LP lock record
    pub async fn create<'c, E>(lock: &NewLpLock, connection: E) -> Result<LpLock, sqlx::Error>
    where
        E: Executor<'c, Database = Postgres>,
    {
        let query = r#"
            INSERT INTO lp_locks (
                token_address, pair_address, lock_contract, lock_contract_name,
                locked_amount, locked_percent, lock_date, unlock_date, tx_hash, block_number
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING *
        "#;

        sqlx::query_as::<_, LpLock>(query)
            .bind(&lock.token_address)
            .bind(&lock.pair_address)
            .bind(&lock.lock_contract)
            .bind(&lock.lock_contract_name)
            .bind(&lock.locked_amount)
            .bind(&lock.locked_percent)
            .bind(lock.lock_date)
            .bind(lock.unlock_date)
            .bind(&lock.tx_hash)
            .bind(lock.block_number)
            .fetch_one(connection)
            .await
    }

    /// Find locks by token address
    pub async fn find_by_token<'c, E>(
        token_address: &str,
        connection: E,
    ) -> Result<Vec<LpLock>, sqlx::Error>
    where
        E: Executor<'c, Database = Postgres>,
    {
        sqlx::query_as::<_, LpLock>(
            "SELECT * FROM lp_locks WHERE token_address = $1 AND is_active = TRUE ORDER BY unlock_date ASC",
        )
        .bind(token_address)
        .fetch_all(connection)
        .await
    }

    /// Find locks by pair address
    pub async fn find_by_pair<'c, E>(
        pair_address: &str,
        connection: E,
    ) -> Result<Vec<LpLock>, sqlx::Error>
    where
        E: Executor<'c, Database = Postgres>,
    {
        sqlx::query_as::<_, LpLock>(
            "SELECT * FROM lp_locks WHERE pair_address = $1 AND is_active = TRUE",
        )
        .bind(pair_address)
        .fetch_all(connection)
        .await
    }

    /// Calculate total locked percent for a token
    pub async fn total_locked_percent<'c, E>(
        token_address: &str,
        connection: E,
    ) -> Result<BigDecimal, sqlx::Error>
    where
        E: Executor<'c, Database = Postgres>,
    {
        let total: Option<BigDecimal> = sqlx::query_scalar(
            "SELECT COALESCE(SUM(locked_percent), 0) FROM lp_locks WHERE token_address = $1 AND is_active = TRUE",
        )
        .bind(token_address)
        .fetch_one(connection)
        .await?;

        Ok(total.unwrap_or_else(|| BigDecimal::from(0)))
    }

    /// Get earliest unlock date for a token
    pub async fn earliest_unlock<'c, E>(
        token_address: &str,
        connection: E,
    ) -> Result<Option<chrono::DateTime<chrono::Utc>>, sqlx::Error>
    where
        E: Executor<'c, Database = Postgres>,
    {
        sqlx::query_scalar(
            "SELECT MIN(unlock_date) FROM lp_locks WHERE token_address = $1 AND is_active = TRUE",
        )
        .bind(token_address)
        .fetch_one(connection)
        .await
    }

    /// Mark a lock as inactive (withdrawn)
    pub async fn deactivate<'c, E>(id: i32, connection: E) -> Result<(), sqlx::Error>
    where
        E: Executor<'c, Database = Postgres>,
    {
        sqlx::query("UPDATE lp_locks SET is_active = FALSE, updated_at = NOW() WHERE id = $1")
            .bind(id)
            .execute(connection)
            .await?;

        Ok(())
    }

    /// Find locks expiring soon (within specified hours)
    pub async fn find_expiring_soon<'c, E>(
        hours: i32,
        connection: E,
    ) -> Result<Vec<LpLock>, sqlx::Error>
    where
        E: Executor<'c, Database = Postgres>,
    {
        sqlx::query_as::<_, LpLock>(
            r#"
            SELECT * FROM lp_locks
            WHERE is_active = TRUE
              AND unlock_date <= NOW() + ($1 || ' hours')::INTERVAL
              AND unlock_date > NOW()
            ORDER BY unlock_date ASC
            "#,
        )
        .bind(hours)
        .fetch_all(connection)
        .await
    }
}
