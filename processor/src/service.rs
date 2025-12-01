use indexer_db::entity::{
    alert::{AlertEvent, AlertType, NewAlert},
    evm_logs::EvmLogs,
    token::Token,
};
use sqlx::{Pool, Postgres};
use std::{env, error::Error};

use crate::{
    defaults,
    events::{self, topics},
    handlers::{self, HandlerContext},
    redis_client::RedisPublisher,
    scoring::bee_score::BeeScoreCalculator,
    utils,
};

/// Create handler context from environment
fn create_handler_context(db_pool: Pool<Postgres>) -> HandlerContext {
    let wbnb_address = env::var("WBNB_ADDRESS")
        .unwrap_or_else(|_| defaults::WBNB_ADDRESS.to_string());
    let busd_address = env::var("BUSD_ADDRESS")
        .unwrap_or_else(|_| defaults::BUSD_ADDRESS.to_string());
    let bnb_price_usd = env::var("BNB_PRICE_USD")
        .unwrap_or_else(|_| defaults::BNB_PRICE_USD.to_string())
        .parse::<f64>()
        .unwrap_or(600.0);
    let whale_threshold_usd = env::var("WHALE_THRESHOLD_USD")
        .unwrap_or_else(|_| defaults::WHALE_THRESHOLD_USD.to_string())
        .parse::<f64>()
        .unwrap_or(5000.0);

    HandlerContext::new(
        db_pool,
        wbnb_address,
        busd_address,
        bnb_price_usd,
        whale_threshold_usd,
    )
}

/// Update token BeeScore and trigger alerts if needed
async fn update_token_score(
    token_address: &str,
    db_pool: &Pool<Postgres>,
) -> Result<(), Box<dyn Error>> {
    // 1. Fetch token with latest metrics
    let token = match Token::find_by_address(token_address, db_pool).await? {
        Some(t) => t,
        None => return Ok(()),
    };

    // 2. Calculate score
    let metrics = token.to_metrics();
    let result = BeeScoreCalculator::calculate(&metrics);

    // 3. Update score in DB
    Token::update_bee_score(
        token_address,
        result.total as i16,
        result.safety_score as i16,
        result.traction_score as i16,
        db_pool,
    )
    .await?;

    // 4. Trigger alert if score is high (>80) and wasn't high before
    if result.total >= 80 {
        let prev_score = token.bee_score.unwrap_or(0);
        if prev_score < 80 {
            let alert = NewAlert {
                alert_type: AlertType::HighBeeScore.as_str().to_string(),
                token_address: Some(token_address.to_string()),
                token_symbol: token.symbol.clone(),
                wallet_address: None,
                title: format!("High BeeScore: {}/100", result.total),
                message: Some(format!(
                    "{} has reached a BeeScore of {}! Safety: {}, Traction: {}",
                    token.symbol.as_deref().unwrap_or("Token"),
                    result.total,
                    result.safety_score,
                    result.traction_score
                )),
                bee_score: Some(result.total as i16),
                amount_usd: None,
                change_percent: None,
                metadata: None,
            };

            if let Err(e) = AlertEvent::create(&alert, db_pool).await {
                eprintln!("Failed to create BeeScore alert: {}", e);
            }
        }
    }

    Ok(())
}

/// Process logs from Postgres, persist to database, and publish to Redis (dual-write)
pub async fn process_logs(
    db_pool: &Pool<Postgres>,
    redis: &mut RedisPublisher,
) -> Result<(), Box<dyn Error>> {
    let batch_size = env::var("BATCH_SIZE")
        .or::<String>(Ok(defaults::BATCH_SIZE.into()))?
        .parse::<i32>()?;

    let unprocessed_logs = EvmLogs::find_all(batch_size, db_pool).await?;

    // Create handler context
    let ctx = create_handler_context(db_pool.clone());

    for log in unprocessed_logs {
        let log_id = log.id;
        let topic0 = format!("0x{}", utils::vec_to_hex(log.event_signature.to_vec()));

        // Try to decode and process
        match events::decode_event(&log) {
            Ok(decoded) => {
                // Process with handler (persist to database)
                match topic0.as_str() {
                    topics::PAIR_CREATED => {
                        let event = events::pair_created::decode(&log)?;
                        if let Err(e) = handlers::pair_created::handle(&ctx, &event).await {
                            eprintln!("PairCreated handler error: {}", e);
                        }
                    }
                    topics::SWAP => {
                        let event = events::swap::decode(&log)?;
                        if let Err(e) = handlers::swap::handle(&ctx, &event).await {
                            eprintln!("Swap handler error: {}", e);
                        } else {
                            // Update score after swap
                            if let Ok(Some(pair)) =
                                indexer_db::entity::pair::Pair::find_by_address(&event.pair, db_pool)
                                    .await
                            {
                                let token_address = pair.get_token_address();
                                if let Err(e) = update_token_score(token_address, db_pool).await {
                                    eprintln!("Failed to update score for {}: {}", token_address, e);
                                }
                            }
                        }
                    }
                    topics::TRANSFER => {
                        let event = events::transfer::decode(&log)?;
                        if let Err(e) = handlers::transfer::handle(&ctx, &event).await {
                            eprintln!("Transfer handler error: {}", e);
                        } else {
                            // Update score after transfer
                            if let Err(e) = update_token_score(&event.token, db_pool).await {
                                eprintln!("Failed to update score for {}: {}", event.token, e);
                            }
                        }
                    }
                    _ => {
                        // Unknown event type, skip handler
                    }
                }

                // Publish to Redis (hot path for real-time updates)
                match redis.publish(decoded.channel, &decoded.payload).await {
                    Ok(_) => {
                        println!(
                            "Published to {}: {} bytes",
                            decoded.channel,
                            decoded.payload.len()
                        );
                    }
                    Err(e) => {
                        eprintln!("Redis publish error: {}", e);
                    }
                }
            }
            Err(e) => {
                eprintln!("Event decode skipped (log_id={}): {}", log_id, e);
            }
        }

        // Delete from Postgres queue (cold path complete)
        if let Err(error) = EvmLogs::delete(log_id, db_pool).await {
            eprintln!("Error deleting log {}: {}", log_id, error);
        }
    }

    Ok(())
}
