use std::env;

use redis::{aio::MultiplexedConnection, AsyncCommands, Client};

use crate::error::AppError;

/// Redis publisher for the hot path (real-time event streaming)
pub struct RedisPublisher {
    connection: MultiplexedConnection,
}

impl RedisPublisher {
    /// Create a new Redis publisher from REDIS_URL environment variable
    pub async fn new() -> Result<Self, AppError> {
        let redis_url = env::var("REDIS_URL")
            .map_err(|_| AppError::MissingEnvVar("REDIS_URL".to_string()))?;

        let client = Client::open(redis_url.as_str())
            .map_err(|e| AppError::RedisConnection(e.to_string()))?;

        let connection = client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| AppError::RedisConnection(e.to_string()))?;

        println!("Connected to Redis at {}", redis_url);
        Ok(Self { connection })
    }

    /// Publish a message to a Redis channel
    pub async fn publish(&mut self, channel: &str, payload: &str) -> Result<(), AppError> {
        self.connection
            .publish::<_, _, ()>(channel, payload)
            .await
            .map_err(|e| AppError::RedisPublish(e.to_string()))?;
        Ok(())
    }
}

/// Redis channels for BeanBee events
pub mod channels {
    /// Channel for new token pair creations
    pub const NEW_PAIR: &str = "chain:events:new_pair";
    /// Channel for swap events (price updates)
    pub const SWAP: &str = "chain:events:swap";
    /// Channel for transfer events (wallet activity)
    pub const TRANSFER: &str = "chain:events:transfer";
}

