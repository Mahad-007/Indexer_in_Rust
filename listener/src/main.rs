//! BeanBee BSC Listener
//!
//! Captures raw blockchain logs from BSC and stores them in PostgreSQL
//! for processing by the processor service.
//!
//! Events tracked:
//! - PairCreated: New token launches on PancakeSwap (MVP)
//! - Swap: Price/volume updates (requires paid RPC)
//! - Transfer: Holder tracking (requires paid RPC)

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
    /// Sync event topic (for reserves/liquidity)
    pub const TOPIC_SYNC: &str = "0x1c411e9a96e071241c2f21f7726b17ae89e3cab4c78be50e062b03a9fffbbad1";
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

    let pancake_factory = env::var("PANCAKESWAP_FACTORY")
        .or_else(|_| env::var("PANCAKE_FACTORY"))
        .unwrap_or_else(|_| defaults::PANCAKE_FACTORY.to_string());

    // For MVP with public RPC: Only index PairCreated events (address-filtered)
    // This detects new token launches on PancakeSwap
    //
    // To enable Swap/Transfer indexing with a paid RPC:
    // 1. Use a paid BSC RPC (e.g., QuickNode, Ankr, etc.)
    // 2. Add additional filter modes for TOPIC_SWAP and TOPIC_TRANSFER
    // 3. Run multiple listener instances or add multi-filter support
    let filter = FilterMode::ByAddressAndTopic {
        address: pancake_factory.clone(),
        topic: topic_pair_created.clone(),
        name: "PairCreated".to_string(),
    };

    let poll_delay = Duration::from_secs(evm_chain.block_time as u64);

    println!("");
    println!("Configuration:");
    println!("  Factory: {}", pancake_factory);
    println!("  Event: PairCreated");
    println!("  Poll Interval: {}s", poll_delay.as_secs());
    println!("");
    println!("Note: Swap/Transfer events require a paid RPC due to volume.");
    println!("      Set up additional listeners for those events.");
    println!("");
    println!("Starting event indexing...");
    println!("");

    loop {
        match fetch_and_save_logs(chain_id, db_pool.clone(), filter.clone()).await {
            Ok(()) => {}
            Err(err) => {
                eprintln!("Indexing error: {:?}", err);
                // Add a small delay on error to prevent rapid retry loops
                sleep(Duration::from_secs(5)).await;
            }
        }
        sleep(poll_delay).await;
    }
}
