use std::{env, time::Duration};

use error::AppError;
use indexer_db::{entity::evm_chains::EvmChains, initialize_database};
use service::{fetch_and_save_logs, FilterMode};
use tokio::time::sleep;

mod error;
mod service;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting BeanBee BSC Indexer...");
    
    let db_pool = initialize_database().await?;
    
    let chain_id_env =
        env::var("CHAIN_ID").map_err(|_| AppError::MissingEnvVar("CHAIN_ID".into()))?;
    let chain_id = chain_id_env
        .parse::<u64>()
        .map_err(|_| AppError::InvalidChainID(chain_id_env))?;

    let evm_chain = EvmChains::fetch_by_id(chain_id, &db_pool).await?;
    println!("Connected to chain: {} (ID: {})", evm_chain.name, chain_id);

    // Get event topics from environment
    let topic_pair_created = env::var("TOPIC_PAIR_CREATED")
        .unwrap_or_else(|_| "0x0d3648bd0f6ba80134a33ba9275ac585d9d315f0ad8355cddefde31afa28d0e9".to_string());
    
    // PancakeSwap V2 Factory address for PairCreated events
    let pancake_factory = env::var("PANCAKE_FACTORY")
        .unwrap_or_else(|_| "0xcA143Ce32Fe78f1f7019d7d551a6402fC5350c73".to_string());

    // For MVP with public RPC: Only index PairCreated events (address-filtered)
    // Swap and Transfer require a paid RPC due to volume
    let filter = FilterMode::ByAddressAndTopic {
        address: pancake_factory,
        topic: topic_pair_created,
        name: "PairCreated".to_string(),
    };

    let poll_delay = Duration::from_secs(evm_chain.block_time as u64);

    println!("Indexing PairCreated events from PancakeSwap V2 Factory...");
    println!("Note: Swap/Transfer events require a paid RPC due to volume.");

    loop {
        match fetch_and_save_logs(chain_id, db_pool.clone(), filter.clone()).await {
            Ok(()) => {}
            Err(err) => {
                eprintln!("Indexing error: {:?}", err);
            }
        }
        sleep(poll_delay).await;
    }
}
