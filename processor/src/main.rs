use indexer_db::{entity::evm_logs::EvmLogs, initialize_database};
use redis_client::RedisPublisher;
use service::process_logs;
use std::{env, error::Error};
use tokio::time::{sleep, Duration};

mod contracts;
mod error;
mod events;
pub mod handlers;
mod redis_client;
pub mod scoring;
mod service;
mod utils;

mod defaults {
    pub const POLL_INTERVAL: &str = "10";
    pub const BATCH_SIZE: &str = "25";
    pub const BNB_PRICE_USD: &str = "600";
    pub const WHALE_THRESHOLD_USD: &str = "5000";
    pub const WBNB_ADDRESS: &str = "0xbb4CdB9CBd36B01bD1cBaEBF2De08d9173bc095c";
    pub const BUSD_ADDRESS: &str = "0xe9e7CEA3DedcA5984780Bafc599bD69ADd087D56";
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("Starting BeanBee Processor (Dual-Write: Postgres + Redis)...");

    // Initialize database connection
    let db_pool = initialize_database().await?;
    println!("Connected to Postgres");

    // Initialize Redis publisher
    let mut redis = RedisPublisher::new().await?;

    let poll_interval = env::var("POLL_INTERVAL")
        .or::<String>(Ok(defaults::POLL_INTERVAL.into()))?
        .parse::<u64>()?;

    let sleep_duration = Duration::from_secs(poll_interval);

    println!("Processor started. Polling every {} seconds...", poll_interval);

    loop {
        let unprocessed_count = match EvmLogs::count(&db_pool).await {
            Ok(count) => count,
            Err(err) => {
                eprintln!(
                    "Error counting unprocessed logs: {err}. Sleeping for {} seconds...",
                    sleep_duration.as_secs()
                );

                sleep(sleep_duration).await;
                continue;
            }
        };

        match unprocessed_count {
            Some(count) => {
                println!("Found {count} unprocessed logs. Processing...");

                if let Err(err) = process_logs(&db_pool, &mut redis).await {
                    eprintln!("Error processing logs: {err}");
                }
            }
            None => {
                println!(
                    "No unprocessed logs. Sleeping for {} seconds...",
                    sleep_duration.as_secs()
                );
                sleep(sleep_duration).await;
            }
        }
    }
}
