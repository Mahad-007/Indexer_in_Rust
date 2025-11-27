
use sqlx::{
    types::{chrono, BigDecimal},
    Executor, Postgres,
};

/// Pair entity representing a DEX trading pair
#[derive(sqlx::FromRow, Debug, Clone)]
pub struct Pair {
    pub id: i32,
    pub address: String,
    pub token0_address: String,
    pub token1_address: String,
    pub factory_address: String,
    pub reserve0: Option<BigDecimal>,
    pub reserve1: Option<BigDecimal>,
    pub base_token_index: Option<i16>, // 0 or 1, indicating which token is WBNB/BUSD
    pub block_number: i64,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub last_updated: Option<chrono::DateTime<chrono::Utc>>,
}

/// Input for creating a new pair
#[derive(Debug, Clone)]
pub struct NewPair {
    pub address: String,
    pub token0_address: String,
    pub token1_address: String,
    pub factory_address: String,
    pub base_token_index: i16,
    pub block_number: i64,
}

impl Pair {
    /// Create a new pair record
    pub async fn create<'c, E>(pair: &NewPair, connection: E) -> Result<Pair, sqlx::Error>
    where
        E: Executor<'c, Database = Postgres>,
    {
        let query = r#"
            INSERT INTO pairs (address, token0_address, token1_address, factory_address, base_token_index, block_number)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (address) DO NOTHING
            RETURNING *
        "#;

        sqlx::query_as::<_, Pair>(query)
            .bind(&pair.address)
            .bind(&pair.token0_address)
            .bind(&pair.token1_address)
            .bind(&pair.factory_address)
            .bind(pair.base_token_index)
            .bind(pair.block_number)
            .fetch_one(connection)
            .await
    }

    /// Find pair by address
    pub async fn find_by_address<'c, E>(
        address: &str,
        connection: E,
    ) -> Result<Option<Pair>, sqlx::Error>
    where
        E: Executor<'c, Database = Postgres>,
    {
        sqlx::query_as::<_, Pair>("SELECT * FROM pairs WHERE address = $1")
            .bind(address)
            .fetch_optional(connection)
            .await
    }

    /// Find pair by token addresses
    pub async fn find_by_tokens<'c, E>(
        token0: &str,
        token1: &str,
        connection: E,
    ) -> Result<Option<Pair>, sqlx::Error>
    where
        E: Executor<'c, Database = Postgres>,
    {
        sqlx::query_as::<_, Pair>(
            r#"
            SELECT * FROM pairs
            WHERE (token0_address = $1 AND token1_address = $2)
               OR (token0_address = $2 AND token1_address = $1)
            "#,
        )
        .bind(token0)
        .bind(token1)
        .fetch_optional(connection)
        .await
    }

    /// Update reserves (from Sync events)
    pub async fn update_reserves<'c, E>(
        address: &str,
        reserve0: &BigDecimal,
        reserve1: &BigDecimal,
        connection: E,
    ) -> Result<(), sqlx::Error>
    where
        E: Executor<'c, Database = Postgres>,
    {
        sqlx::query(
            r#"
            UPDATE pairs SET
                reserve0 = $2,
                reserve1 = $3,
                last_updated = NOW()
            WHERE address = $1
            "#,
        )
        .bind(address)
        .bind(reserve0)
        .bind(reserve1)
        .execute(connection)
        .await?;

        Ok(())
    }

    /// Get recent pairs (newest token launches)
    pub async fn find_recent<'c, E>(limit: i32, connection: E) -> Result<Vec<Pair>, sqlx::Error>
    where
        E: Executor<'c, Database = Postgres>,
    {
        sqlx::query_as::<_, Pair>("SELECT * FROM pairs ORDER BY created_at DESC LIMIT $1")
            .bind(limit)
            .fetch_all(connection)
            .await
    }

    /// Get the non-base token address (the memecoin, not WBNB)
    pub fn get_token_address(&self) -> &str {
        match self.base_token_index {
            Some(0) => &self.token1_address,
            Some(1) => &self.token0_address,
            _ => &self.token0_address, // Default fallback
        }
    }

    /// Get the base token address (WBNB/BUSD)
    pub fn get_base_address(&self) -> &str {
        match self.base_token_index {
            Some(0) => &self.token0_address,
            Some(1) => &self.token1_address,
            _ => &self.token1_address, // Default fallback
        }
    }
}
