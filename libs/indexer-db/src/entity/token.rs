
use sqlx::{
    types::{chrono, BigDecimal},
    Executor, Postgres,
};

/// Token entity representing a BEP-20 token tracked by BeanBee
#[derive(sqlx::FromRow, Debug, Clone)]
pub struct Token {
    pub id: i32,
    pub address: String,
    pub name: Option<String>,
    pub symbol: Option<String>,
    pub decimals: Option<i16>,
    pub total_supply: Option<BigDecimal>,
    pub pair_address: Option<String>,
    pub creator_address: Option<String>,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub block_number: Option<i64>,

    // Live metrics
    pub price_usd: Option<BigDecimal>,
    pub price_bnb: Option<BigDecimal>,
    pub price_change_1h: Option<BigDecimal>,
    pub price_change_24h: Option<BigDecimal>,
    pub market_cap_usd: Option<BigDecimal>,
    pub liquidity_usd: Option<BigDecimal>,
    pub liquidity_bnb: Option<BigDecimal>,
    pub volume_1h_usd: Option<BigDecimal>,
    pub volume_24h_usd: Option<BigDecimal>,
    pub trades_1h: Option<i32>,
    pub trades_24h: Option<i32>,
    pub buys_1h: Option<i32>,
    pub sells_1h: Option<i32>,

    // Holder metrics
    pub holder_count: Option<i32>,
    pub holder_count_1h_ago: Option<i32>,
    pub top_10_holder_percent: Option<BigDecimal>,
    pub dev_holdings_percent: Option<BigDecimal>,
    pub sniper_ratio: Option<BigDecimal>,

    // Safety flags
    pub lp_locked: Option<bool>,
    pub lp_lock_percent: Option<BigDecimal>,
    pub lp_unlock_date: Option<chrono::DateTime<chrono::Utc>>,
    pub ownership_renounced: Option<bool>,

    // BeeScore
    pub bee_score: Option<i16>,
    pub safety_score: Option<i16>,
    pub traction_score: Option<i16>,

    pub last_updated: Option<chrono::DateTime<chrono::Utc>>,
    pub indexed_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Input for creating a new token
#[derive(Debug, Clone)]
pub struct NewToken {
    pub address: String,
    pub name: Option<String>,
    pub symbol: Option<String>,
    pub decimals: Option<i16>,
    pub total_supply: Option<BigDecimal>,
    pub pair_address: Option<String>,
    pub creator_address: Option<String>,
    pub block_number: Option<i64>,
}

/// Token metrics for BeeScore calculation
#[derive(Debug, Clone, Default)]
pub struct TokenMetrics {
    pub liquidity_usd: f64,
    pub lp_locked: bool,
    pub lp_lock_percent: f64,
    pub top_10_holder_percent: f64,
    pub dev_holdings_percent: f64,
    pub ownership_renounced: bool,
    pub volume_1h_usd: f64,
    pub trades_1h: i32,
    pub holder_count: i32,
    pub holder_count_1h_ago: i32,
    pub price_change_1h: f64,
    pub buys_1h: i32,
    pub sells_1h: i32,
}

impl Token {
    /// Create a new token record
    pub async fn create<'c, E>(token: &NewToken, connection: E) -> Result<Token, sqlx::Error>
    where
        E: Executor<'c, Database = Postgres>,
    {
        let query = r#"
            INSERT INTO tokens (address, name, symbol, decimals, total_supply, pair_address, creator_address, block_number, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW())
            ON CONFLICT (address) DO UPDATE SET
                name = COALESCE(EXCLUDED.name, tokens.name),
                symbol = COALESCE(EXCLUDED.symbol, tokens.symbol),
                decimals = COALESCE(EXCLUDED.decimals, tokens.decimals),
                total_supply = COALESCE(EXCLUDED.total_supply, tokens.total_supply),
                pair_address = COALESCE(EXCLUDED.pair_address, tokens.pair_address),
                last_updated = NOW()
            RETURNING *
        "#;

        sqlx::query_as::<_, Token>(query)
            .bind(&token.address)
            .bind(&token.name)
            .bind(&token.symbol)
            .bind(token.decimals)
            .bind(&token.total_supply)
            .bind(&token.pair_address)
            .bind(&token.creator_address)
            .bind(token.block_number)
            .fetch_one(connection)
            .await
    }

    /// Find token by address
    pub async fn find_by_address<'c, E>(
        address: &str,
        connection: E,
    ) -> Result<Option<Token>, sqlx::Error>
    where
        E: Executor<'c, Database = Postgres>,
    {
        sqlx::query_as::<_, Token>("SELECT * FROM tokens WHERE address = $1")
            .bind(address)
            .fetch_optional(connection)
            .await
    }

    /// Find token by pair address
    pub async fn find_by_pair_address<'c, E>(
        pair_address: &str,
        connection: E,
    ) -> Result<Option<Token>, sqlx::Error>
    where
        E: Executor<'c, Database = Postgres>,
    {
        sqlx::query_as::<_, Token>("SELECT * FROM tokens WHERE pair_address = $1")
            .bind(pair_address)
            .fetch_optional(connection)
            .await
    }

    /// Get newest tokens (for /api/tokens/new)
    pub async fn find_newest<'c, E>(
        limit: i32,
        connection: E,
    ) -> Result<Vec<Token>, sqlx::Error>
    where
        E: Executor<'c, Database = Postgres>,
    {
        sqlx::query_as::<_, Token>(
            "SELECT * FROM tokens ORDER BY created_at DESC NULLS LAST LIMIT $1",
        )
        .bind(limit)
        .fetch_all(connection)
        .await
    }

    /// Get hot tokens (sorted by volume and bee_score)
    pub async fn find_hot<'c, E>(
        limit: i32,
        connection: E,
    ) -> Result<Vec<Token>, sqlx::Error>
    where
        E: Executor<'c, Database = Postgres>,
    {
        sqlx::query_as::<_, Token>(
            r#"
            SELECT * FROM tokens
            WHERE volume_1h_usd > 0 OR bee_score > 0
            ORDER BY (COALESCE(volume_1h_usd, 0) + COALESCE(bee_score, 0) * 100) DESC
            LIMIT $1
            "#,
        )
        .bind(limit)
        .fetch_all(connection)
        .await
    }

    /// Update token price and volume metrics
    pub async fn update_price_metrics<'c, E>(
        address: &str,
        price_usd: &BigDecimal,
        price_bnb: &BigDecimal,
        liquidity_usd: &BigDecimal,
        liquidity_bnb: &BigDecimal,
        connection: E,
    ) -> Result<(), sqlx::Error>
    where
        E: Executor<'c, Database = Postgres>,
    {
        sqlx::query(
            r#"
            UPDATE tokens SET
                price_usd = $2,
                price_bnb = $3,
                liquidity_usd = $4,
                liquidity_bnb = $5,
                last_updated = NOW()
            WHERE address = $1
            "#,
        )
        .bind(address)
        .bind(price_usd)
        .bind(price_bnb)
        .bind(liquidity_usd)
        .bind(liquidity_bnb)
        .execute(connection)
        .await?;

        Ok(())
    }

    /// Increment trade counters
    pub async fn increment_trade_count<'c, E>(
        address: &str,
        is_buy: bool,
        amount_usd: &BigDecimal,
        connection: E,
    ) -> Result<(), sqlx::Error>
    where
        E: Executor<'c, Database = Postgres>,
    {
        let query = if is_buy {
            r#"
            UPDATE tokens SET
                trades_1h = COALESCE(trades_1h, 0) + 1,
                trades_24h = COALESCE(trades_24h, 0) + 1,
                buys_1h = COALESCE(buys_1h, 0) + 1,
                volume_1h_usd = COALESCE(volume_1h_usd, 0) + $2,
                volume_24h_usd = COALESCE(volume_24h_usd, 0) + $2,
                last_updated = NOW()
            WHERE address = $1
            "#
        } else {
            r#"
            UPDATE tokens SET
                trades_1h = COALESCE(trades_1h, 0) + 1,
                trades_24h = COALESCE(trades_24h, 0) + 1,
                sells_1h = COALESCE(sells_1h, 0) + 1,
                volume_1h_usd = COALESCE(volume_1h_usd, 0) + $2,
                volume_24h_usd = COALESCE(volume_24h_usd, 0) + $2,
                last_updated = NOW()
            WHERE address = $1
            "#
        };

        sqlx::query(query)
            .bind(address)
            .bind(amount_usd)
            .execute(connection)
            .await?;

        Ok(())
    }

    /// Update BeeScore
    pub async fn update_bee_score<'c, E>(
        address: &str,
        bee_score: i16,
        safety_score: i16,
        traction_score: i16,
        connection: E,
    ) -> Result<(), sqlx::Error>
    where
        E: Executor<'c, Database = Postgres>,
    {
        sqlx::query(
            r#"
            UPDATE tokens SET
                bee_score = $2,
                safety_score = $3,
                traction_score = $4,
                last_updated = NOW()
            WHERE address = $1
            "#,
        )
        .bind(address)
        .bind(bee_score)
        .bind(safety_score)
        .bind(traction_score)
        .execute(connection)
        .await?;

        Ok(())
    }

    /// Update holder metrics
    pub async fn update_holder_metrics<'c, E>(
        address: &str,
        holder_count: i32,
        top_10_percent: &BigDecimal,
        dev_percent: &BigDecimal,
        sniper_ratio: &BigDecimal,
        connection: E,
    ) -> Result<(), sqlx::Error>
    where
        E: Executor<'c, Database = Postgres>,
    {
        sqlx::query(
            r#"
            UPDATE tokens SET
                holder_count = $2,
                top_10_holder_percent = $3,
                dev_holdings_percent = $4,
                sniper_ratio = $5,
                last_updated = NOW()
            WHERE address = $1
            "#,
        )
        .bind(address)
        .bind(holder_count)
        .bind(top_10_percent)
        .bind(dev_percent)
        .bind(sniper_ratio)
        .execute(connection)
        .await?;

        Ok(())
    }

    /// Update LP lock status
    pub async fn update_lp_lock<'c, E>(
        address: &str,
        lp_locked: bool,
        lp_lock_percent: &BigDecimal,
        unlock_date: Option<chrono::DateTime<chrono::Utc>>,
        connection: E,
    ) -> Result<(), sqlx::Error>
    where
        E: Executor<'c, Database = Postgres>,
    {
        sqlx::query(
            r#"
            UPDATE tokens SET
                lp_locked = $2,
                lp_lock_percent = $3,
                lp_unlock_date = $4,
                last_updated = NOW()
            WHERE address = $1
            "#,
        )
        .bind(address)
        .bind(lp_locked)
        .bind(lp_lock_percent)
        .bind(unlock_date)
        .execute(connection)
        .await?;

        Ok(())
    }

    /// Convert to TokenMetrics for BeeScore calculation
    pub fn to_metrics(&self) -> TokenMetrics {
        TokenMetrics {
            liquidity_usd: self
                .liquidity_usd
                .as_ref()
                .and_then(|v| v.to_string().parse().ok())
                .unwrap_or(0.0),
            lp_locked: self.lp_locked.unwrap_or(false),
            lp_lock_percent: self
                .lp_lock_percent
                .as_ref()
                .and_then(|v| v.to_string().parse().ok())
                .unwrap_or(0.0),
            top_10_holder_percent: self
                .top_10_holder_percent
                .as_ref()
                .and_then(|v| v.to_string().parse().ok())
                .unwrap_or(100.0),
            dev_holdings_percent: self
                .dev_holdings_percent
                .as_ref()
                .and_then(|v| v.to_string().parse().ok())
                .unwrap_or(100.0),
            ownership_renounced: self.ownership_renounced.unwrap_or(false),
            volume_1h_usd: self
                .volume_1h_usd
                .as_ref()
                .and_then(|v| v.to_string().parse().ok())
                .unwrap_or(0.0),
            trades_1h: self.trades_1h.unwrap_or(0),
            holder_count: self.holder_count.unwrap_or(0),
            holder_count_1h_ago: self.holder_count_1h_ago.unwrap_or(0),
            price_change_1h: self
                .price_change_1h
                .as_ref()
                .and_then(|v| v.to_string().parse().ok())
                .unwrap_or(0.0),
            buys_1h: self.buys_1h.unwrap_or(0),
            sells_1h: self.sells_1h.unwrap_or(0),
        }
    }
}
