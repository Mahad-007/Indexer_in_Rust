use indexer_db::entity::evm_logs::EvmLogs;
use sqlx::{Pool, Postgres};
use std::{env, error::Error};

use crate::{
    defaults,
    events::{self, topics},
    handlers::{self, HandlerContext},
    redis_client::RedisPublisher,
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
                        }
                    }
                    topics::TRANSFER => {
                        let event = events::transfer::decode(&log)?;
                        if let Err(e) = handlers::transfer::handle(&ctx, &event).await {
                            eprintln!("Transfer handler error: {}", e);
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
                        // Continue processing - don't fail the whole batch for Redis errors
                    }
                }
            }
            Err(e) => {
                // Log unknown events but don't fail
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
