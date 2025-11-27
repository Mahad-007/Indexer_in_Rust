
use serde_json::Value as JsonValue;
use sqlx::{
    types::{chrono, BigDecimal, Json},
    Executor, Postgres,
};

/// AlertEvent entity for notification queue
#[derive(sqlx::FromRow, Debug, Clone)]
pub struct AlertEvent {
    pub id: i32,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub alert_type: String,
    pub token_address: Option<String>,
    pub token_symbol: Option<String>,
    pub wallet_address: Option<String>,
    pub title: String,
    pub message: Option<String>,
    pub bee_score: Option<i16>,
    pub amount_usd: Option<BigDecimal>,
    pub change_percent: Option<BigDecimal>,
    pub metadata: Option<Json<JsonValue>>,
    pub processed: Option<bool>,
    pub processed_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Alert types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertType {
    NewToken,
    WhaleBuy,
    WhaleSell,
    PricePump,
    PriceDump,
    LpLocked,
    LpUnlocking,
    HighBeeScore,
    DevSell,
}

impl AlertType {
    pub fn as_str(&self) -> &'static str {
        match self {
            AlertType::NewToken => "new_token",
            AlertType::WhaleBuy => "whale_buy",
            AlertType::WhaleSell => "whale_sell",
            AlertType::PricePump => "price_pump",
            AlertType::PriceDump => "price_dump",
            AlertType::LpLocked => "lp_locked",
            AlertType::LpUnlocking => "lp_unlocking",
            AlertType::HighBeeScore => "high_bee_score",
            AlertType::DevSell => "dev_sell",
        }
    }
}

/// Input for creating a new alert
#[derive(Debug, Clone)]
pub struct NewAlert {
    pub alert_type: String,
    pub token_address: Option<String>,
    pub token_symbol: Option<String>,
    pub wallet_address: Option<String>,
    pub title: String,
    pub message: Option<String>,
    pub bee_score: Option<i16>,
    pub amount_usd: Option<BigDecimal>,
    pub change_percent: Option<BigDecimal>,
    pub metadata: Option<JsonValue>,
}

impl AlertEvent {
    /// Create a new alert event
    pub async fn create<'c, E>(alert: &NewAlert, connection: E) -> Result<AlertEvent, sqlx::Error>
    where
        E: Executor<'c, Database = Postgres>,
    {
        let query = r#"
            INSERT INTO alert_events (
                alert_type, token_address, token_symbol, wallet_address,
                title, message, bee_score, amount_usd, change_percent, metadata
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING *
        "#;

        sqlx::query_as::<_, AlertEvent>(query)
            .bind(&alert.alert_type)
            .bind(&alert.token_address)
            .bind(&alert.token_symbol)
            .bind(&alert.wallet_address)
            .bind(&alert.title)
            .bind(&alert.message)
            .bind(alert.bee_score)
            .bind(&alert.amount_usd)
            .bind(&alert.change_percent)
            .bind(alert.metadata.as_ref().map(Json))
            .fetch_one(connection)
            .await
    }

    /// Create a new token alert
    pub async fn create_new_token_alert<'c, E>(
        token_address: &str,
        token_symbol: &str,
        connection: E,
    ) -> Result<AlertEvent, sqlx::Error>
    where
        E: Executor<'c, Database = Postgres>,
    {
        let alert = NewAlert {
            alert_type: AlertType::NewToken.as_str().to_string(),
            token_address: Some(token_address.to_string()),
            token_symbol: Some(token_symbol.to_string()),
            wallet_address: None,
            title: format!("New Token: {}", token_symbol),
            message: Some(format!("New token {} launched on PancakeSwap", token_symbol)),
            bee_score: None,
            amount_usd: None,
            change_percent: None,
            metadata: None,
        };

        Self::create(&alert, connection).await
    }

    /// Create a whale alert
    pub async fn create_whale_alert<'c, E>(
        token_address: &str,
        token_symbol: &str,
        wallet_address: &str,
        is_buy: bool,
        amount_usd: &BigDecimal,
        connection: E,
    ) -> Result<AlertEvent, sqlx::Error>
    where
        E: Executor<'c, Database = Postgres>,
    {
        let alert_type = if is_buy {
            AlertType::WhaleBuy
        } else {
            AlertType::WhaleSell
        };

        let action = if is_buy { "bought" } else { "sold" };

        let alert = NewAlert {
            alert_type: alert_type.as_str().to_string(),
            token_address: Some(token_address.to_string()),
            token_symbol: Some(token_symbol.to_string()),
            wallet_address: Some(wallet_address.to_string()),
            title: format!("Whale {} ${}", action, token_symbol),
            message: Some(format!(
                "Whale {} ${} worth of {}",
                action, amount_usd, token_symbol
            )),
            bee_score: None,
            amount_usd: Some(amount_usd.clone()),
            change_percent: None,
            metadata: None,
        };

        Self::create(&alert, connection).await
    }

    /// Get unprocessed alerts
    pub async fn find_unprocessed<'c, E>(
        limit: i32,
        connection: E,
    ) -> Result<Vec<AlertEvent>, sqlx::Error>
    where
        E: Executor<'c, Database = Postgres>,
    {
        sqlx::query_as::<_, AlertEvent>(
            "SELECT * FROM alert_events WHERE processed = FALSE ORDER BY created_at ASC LIMIT $1",
        )
        .bind(limit)
        .fetch_all(connection)
        .await
    }

    /// Get recent alerts (for feed)
    pub async fn find_recent<'c, E>(
        limit: i32,
        connection: E,
    ) -> Result<Vec<AlertEvent>, sqlx::Error>
    where
        E: Executor<'c, Database = Postgres>,
    {
        sqlx::query_as::<_, AlertEvent>(
            "SELECT * FROM alert_events ORDER BY created_at DESC LIMIT $1",
        )
        .bind(limit)
        .fetch_all(connection)
        .await
    }

    /// Get alerts by type
    pub async fn find_by_type<'c, E>(
        alert_type: &str,
        limit: i32,
        connection: E,
    ) -> Result<Vec<AlertEvent>, sqlx::Error>
    where
        E: Executor<'c, Database = Postgres>,
    {
        sqlx::query_as::<_, AlertEvent>(
            "SELECT * FROM alert_events WHERE alert_type = $1 ORDER BY created_at DESC LIMIT $2",
        )
        .bind(alert_type)
        .bind(limit)
        .fetch_all(connection)
        .await
    }

    /// Get alerts for a token
    pub async fn find_by_token<'c, E>(
        token_address: &str,
        limit: i32,
        connection: E,
    ) -> Result<Vec<AlertEvent>, sqlx::Error>
    where
        E: Executor<'c, Database = Postgres>,
    {
        sqlx::query_as::<_, AlertEvent>(
            "SELECT * FROM alert_events WHERE token_address = $1 ORDER BY created_at DESC LIMIT $2",
        )
        .bind(token_address)
        .bind(limit)
        .fetch_all(connection)
        .await
    }

    /// Mark alert as processed
    pub async fn mark_processed<'c, E>(id: i32, connection: E) -> Result<(), sqlx::Error>
    where
        E: Executor<'c, Database = Postgres>,
    {
        sqlx::query(
            "UPDATE alert_events SET processed = TRUE, processed_at = NOW() WHERE id = $1",
        )
        .bind(id)
        .execute(connection)
        .await?;

        Ok(())
    }

    /// Mark multiple alerts as processed
    pub async fn mark_many_processed<'c, E>(
        ids: &[i32],
        connection: E,
    ) -> Result<(), sqlx::Error>
    where
        E: Executor<'c, Database = Postgres>,
    {
        sqlx::query(
            "UPDATE alert_events SET processed = TRUE, processed_at = NOW() WHERE id = ANY($1)",
        )
        .bind(ids)
        .execute(connection)
        .await?;

        Ok(())
    }
}
