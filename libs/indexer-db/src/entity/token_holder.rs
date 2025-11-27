
use sqlx::{
    types::{chrono, BigDecimal},
    Executor, Postgres,
};

/// TokenHolder entity representing a wallet holding a token
#[derive(sqlx::FromRow, Debug, Clone)]
pub struct TokenHolder {
    pub id: i32,
    pub token_address: String,
    pub wallet_address: String,
    pub balance: Option<BigDecimal>,
    pub percent_of_supply: Option<BigDecimal>,
    pub is_dev: Option<bool>,
    pub is_sniper: Option<bool>,
    pub is_contract: Option<bool>,
    pub first_buy_block: Option<i64>,
    pub last_updated: Option<chrono::DateTime<chrono::Utc>>,
}

/// Input for creating/updating a token holder
#[derive(Debug, Clone)]
pub struct NewTokenHolder {
    pub token_address: String,
    pub wallet_address: String,
    pub balance: BigDecimal,
    pub is_dev: bool,
    pub is_sniper: bool,
    pub is_contract: bool,
    pub first_buy_block: Option<i64>,
}

impl TokenHolder {
    /// Create or update a token holder record
    pub async fn upsert<'c, E>(holder: &NewTokenHolder, connection: E) -> Result<TokenHolder, sqlx::Error>
    where
        E: Executor<'c, Database = Postgres>,
    {
        let query = r#"
            INSERT INTO token_holders (token_address, wallet_address, balance, is_dev, is_sniper, is_contract, first_buy_block)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (token_address, wallet_address) DO UPDATE SET
                balance = EXCLUDED.balance,
                is_dev = token_holders.is_dev OR EXCLUDED.is_dev,
                is_sniper = token_holders.is_sniper OR EXCLUDED.is_sniper,
                is_contract = token_holders.is_contract OR EXCLUDED.is_contract,
                first_buy_block = COALESCE(token_holders.first_buy_block, EXCLUDED.first_buy_block),
                last_updated = NOW()
            RETURNING *
        "#;

        sqlx::query_as::<_, TokenHolder>(query)
            .bind(&holder.token_address)
            .bind(&holder.wallet_address)
            .bind(&holder.balance)
            .bind(holder.is_dev)
            .bind(holder.is_sniper)
            .bind(holder.is_contract)
            .bind(holder.first_buy_block)
            .fetch_one(connection)
            .await
    }

    /// Update holder balance
    pub async fn update_balance<'c, E>(
        token_address: &str,
        wallet_address: &str,
        balance: &BigDecimal,
        connection: E,
    ) -> Result<(), sqlx::Error>
    where
        E: Executor<'c, Database = Postgres>,
    {
        sqlx::query(
            r#"
            INSERT INTO token_holders (token_address, wallet_address, balance)
            VALUES ($1, $2, $3)
            ON CONFLICT (token_address, wallet_address) DO UPDATE SET
                balance = EXCLUDED.balance,
                last_updated = NOW()
            "#,
        )
        .bind(token_address)
        .bind(wallet_address)
        .bind(balance)
        .execute(connection)
        .await?;

        Ok(())
    }

    /// Get top holders for a token
    pub async fn find_top_holders<'c, E>(
        token_address: &str,
        limit: i32,
        connection: E,
    ) -> Result<Vec<TokenHolder>, sqlx::Error>
    where
        E: Executor<'c, Database = Postgres>,
    {
        sqlx::query_as::<_, TokenHolder>(
            "SELECT * FROM token_holders WHERE token_address = $1 ORDER BY balance DESC NULLS LAST LIMIT $2",
        )
        .bind(token_address)
        .bind(limit)
        .fetch_all(connection)
        .await
    }

    /// Count holders for a token
    pub async fn count_holders<'c, E>(
        token_address: &str,
        connection: E,
    ) -> Result<i64, sqlx::Error>
    where
        E: Executor<'c, Database = Postgres>,
    {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM token_holders WHERE token_address = $1 AND balance > 0",
        )
        .bind(token_address)
        .fetch_one(connection)
        .await?;

        Ok(count)
    }

    /// Get dev holders for a token
    pub async fn find_dev_holders<'c, E>(
        token_address: &str,
        connection: E,
    ) -> Result<Vec<TokenHolder>, sqlx::Error>
    where
        E: Executor<'c, Database = Postgres>,
    {
        sqlx::query_as::<_, TokenHolder>(
            "SELECT * FROM token_holders WHERE token_address = $1 AND is_dev = TRUE",
        )
        .bind(token_address)
        .fetch_all(connection)
        .await
    }

    /// Get sniper holders for a token
    pub async fn find_sniper_holders<'c, E>(
        token_address: &str,
        connection: E,
    ) -> Result<Vec<TokenHolder>, sqlx::Error>
    where
        E: Executor<'c, Database = Postgres>,
    {
        sqlx::query_as::<_, TokenHolder>(
            "SELECT * FROM token_holders WHERE token_address = $1 AND is_sniper = TRUE",
        )
        .bind(token_address)
        .fetch_all(connection)
        .await
    }

    /// Calculate top 10 holders percentage
    pub async fn calculate_top_10_percent<'c, E>(
        token_address: &str,
        connection: E,
    ) -> Result<BigDecimal, sqlx::Error>
    where
        E: Executor<'c, Database = Postgres>,
    {
        let percent: Option<BigDecimal> = sqlx::query_scalar(
            r#"
            SELECT COALESCE(SUM(percent_of_supply), 0)
            FROM (
                SELECT percent_of_supply
                FROM token_holders
                WHERE token_address = $1 AND balance > 0
                ORDER BY balance DESC
                LIMIT 10
            ) top10
            "#,
        )
        .bind(token_address)
        .fetch_one(connection)
        .await?;

        Ok(percent.unwrap_or_else(|| BigDecimal::from(0)))
    }

    /// Mark holder as dev
    pub async fn mark_as_dev<'c, E>(
        token_address: &str,
        wallet_address: &str,
        connection: E,
    ) -> Result<(), sqlx::Error>
    where
        E: Executor<'c, Database = Postgres>,
    {
        sqlx::query(
            "UPDATE token_holders SET is_dev = TRUE WHERE token_address = $1 AND wallet_address = $2",
        )
        .bind(token_address)
        .bind(wallet_address)
        .execute(connection)
        .await?;

        Ok(())
    }

    /// Mark holder as sniper
    pub async fn mark_as_sniper<'c, E>(
        token_address: &str,
        wallet_address: &str,
        connection: E,
    ) -> Result<(), sqlx::Error>
    where
        E: Executor<'c, Database = Postgres>,
    {
        sqlx::query(
            "UPDATE token_holders SET is_sniper = TRUE WHERE token_address = $1 AND wallet_address = $2",
        )
        .bind(token_address)
        .bind(wallet_address)
        .execute(connection)
        .await?;

        Ok(())
    }

    /// Update percent of supply for all holders of a token
    pub async fn recalculate_percentages<'c, E>(
        token_address: &str,
        total_supply: &BigDecimal,
        connection: E,
    ) -> Result<(), sqlx::Error>
    where
        E: Executor<'c, Database = Postgres>,
    {
        sqlx::query(
            r#"
            UPDATE token_holders SET
                percent_of_supply = (balance / $2) * 100,
                last_updated = NOW()
            WHERE token_address = $1 AND balance > 0
            "#,
        )
        .bind(token_address)
        .bind(total_supply)
        .execute(connection)
        .await?;

        Ok(())
    }
}
