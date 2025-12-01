//! BeanBee BSC Listener
//!
//! Captures raw blockchain logs from BSC and stores them in PostgreSQL
//! for processing by the processor service.
//!
//! Events tracked:
//! - PairCreated: New token launches on PancakeSwap
//! - Swap: Price/volume updates (requires paid RPC for full chain)
//! - Transfer: Holder tracking (requires paid RPC for full chain)

use std::{env, time::Duration};

use error::AppError;
use indexer_db::{entity::evm_chains::EvmChains, initialize_database};
use service::{fetch_and_save_logs, FilterMode};
use tokio::time::sleep;

mod error;
mod service;

/// Default addresses and topics for BSC
mod defaults {
    /// PancakeSwap V2 Factory on BSC
    pub const PANCAKE_FACTORY: &str = "0xcA143Ce32Fe78f1f7019d7d551a6402fC5350c73";
    /// PairCreated event topic
    pub const TOPIC_PAIR_CREATED: &str = "0x0d3648bd0f6ba80134a33ba9275ac585d9d315f0ad8355cddefde31afa28d0e9";
    /// Swap event topic
    pub const TOPIC_SWAP: &str = "0xd78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822";
    /// Transfer event topic
    pub const TOPIC_TRANSFER: &str = "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef";
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("============================================");
    println!("   BeanBee BSC Listener - Alpha Discovery   ");
    println!("============================================");

    let db_pool = initialize_database().await?;
    println!("Connected to PostgreSQL");

    let chain_id_env =
        env::var("CHAIN_ID").map_err(|_| AppError::MissingEnvVar("CHAIN_ID".into()))?;
    let chain_id = chain_id_env
        .parse::<u64>()
        .map_err(|_| AppError::InvalidChainID(chain_id_env))?;

    let evm_chain = EvmChains::fetch_by_id(chain_id, &db_pool).await?;
    println!("Chain: {} (ID: {})", evm_chain.name, chain_id);

    // Get configuration from environment
    let topic_pair_created = env::var("TOPIC_PAIR_CREATED")
        .unwrap_or_else(|_| defaults::TOPIC_PAIR_CREATED.to_string());
    
    let topic_swap = env::var("TOPIC_SWAP")
        .unwrap_or_else(|_| defaults::TOPIC_SWAP.to_string());
        
    let topic_transfer = env::var("TOPIC_TRANSFER")
        .unwrap_or_else(|_| defaults::TOPIC_TRANSFER.to_string());

    let pancake_factory = env::var("PANCAKESWAP_FACTORY")
        .or_else(|_| env::var("PANCAKE_FACTORY"))
        .unwrap_or_else(|_| defaults::PANCAKE_FACTORY.to_string());

    let poll_delay = Duration::from_secs(evm_chain.block_time as u64);

    println!("");
    println!("Starting event listeners...");
    println!("  Poll Interval: {}s", poll_delay.as_secs());
    println!("");

    // 1. PairCreated Listener (New Tokens)
    let db_pool_1 = db_pool.clone();
    let filter_pair = FilterMode::ByAddressAndTopic {
        address: pancake_factory.clone(),
        topic: topic_pair_created.clone(),
        name: "PairCreated".to_string(),
    };
    
    let handle_pair = tokio::spawn(async move {
        println!("Started PairCreated listener");
        loop {
            match fetch_and_save_logs(chain_id, db_pool_1.clone(), filter_pair.clone()).await {
                Ok(()) => {}
                Err(err) => {
                    eprintln!("PairCreated listener error: {:?}", err);
                    sleep(Duration::from_secs(5)).await;
                }
            }
            sleep(poll_delay).await;
        }
    });

    // 2. Swap Listener (Price, Volume, Whales)
    // UNCOMMENT FOR PRODUCTION WITH PAID RPC
    /*
    let db_pool_2 = db_pool.clone();
    let filter_swap = FilterMode::ByTopic {
        topic: topic_swap.clone(),
        name: "Swap".to_string(),
    };

    let handle_swap = tokio::spawn(async move {
        println!("Started Swap listener (Global)");
        loop {
            match fetch_and_save_logs(chain_id, db_pool_2.clone(), filter_swap.clone()).await {
                Ok(()) => {}
                Err(err) => {
                    eprintln!("Swap listener error: {:?}", err);
                    sleep(Duration::from_secs(5)).await;
                }
            }
            sleep(poll_delay).await;
        }
    });
    */

    // 3. Transfer Listener (Holders)
    // UNCOMMENT FOR PRODUCTION WITH PAID RPC
    /*
    let db_pool_3 = db_pool.clone();
    let filter_transfer = FilterMode::ByTopic {
        topic: topic_transfer.clone(),
        name: "Transfer".to_string(),
    };

    let handle_transfer = tokio::spawn(async move {
        println!("Started Transfer listener (Global)");
        loop {
            match fetch_and_save_logs(chain_id, db_pool_3.clone(), filter_transfer.clone()).await {
                Ok(()) => {}
                Err(err) => {
                    eprintln!("Transfer listener error: {:?}", err);
                    sleep(Duration::from_secs(5)).await;
                }
            }
            sleep(poll_delay).await;
        }
    });
    */
    
    println!("NOTE: Swap and Transfer listeners are disabled by default to prevent RPC rate limits.");
    println!("      To enable full 'Live Feed' data (Whales, Scores, Pumps), uncomment the listeners in listener/src/main.rs");
    println!("      and ensure you are using a paid RPC provider.");

    // Wait for all tasks (they run forever)
    // let _ = tokio::join!(handle_pair, handle_swap, handle_transfer);
    let _ = tokio::join!(handle_pair);

    Ok(())
}
