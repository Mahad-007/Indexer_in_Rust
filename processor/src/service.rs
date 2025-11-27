use indexer_db::entity::evm_logs::EvmLogs;
use sqlx::{Pool, Postgres};
use std::{env, error::Error};

use crate::{defaults, events, redis_client::RedisPublisher};

/// Process logs from Postgres and publish to Redis (dual-write)
pub async fn process_logs(
    db_pool: &Pool<Postgres>,
    redis: &mut RedisPublisher,
) -> Result<(), Box<dyn Error>> {
    let batch_size = env::var("BATCH_SIZE")
        .or::<String>(Ok(defaults::BATCH_SIZE.into()))?
        .parse::<i32>()?;

    let unprocessed_logs = EvmLogs::find_all(batch_size, db_pool).await?;

    for log in unprocessed_logs {
        let log_id = log.id;
        
        // Try to decode and publish to Redis
        match events::decode_event(&log) {
            Ok(decoded) => {
                // Publish to Redis (hot path)
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
