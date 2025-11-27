use std::{
    env,
    error::Error,
    future::Future,
    pin::Pin,
    str::FromStr,
    task::{Context, Poll},
    time::Duration,
};

use alloy::{
    eips::BlockNumberOrTag,
    primitives::{Address, FixedBytes},
    providers::{Provider, ProviderBuilder},
    rpc::types::{Filter, Log},
};
use indexer_db::entity::{evm_logs::EvmLogs, evm_sync_logs::EvmSyncLogs};
use sqlx::{Pool, Postgres};
use tokio::time::sleep;
use tower::Service;

use crate::error::AppError;

mod defaults {
    pub const RPC_DELAY_MS: &str = "5000";  // 5 seconds between calls for public BSC RPC
    pub const MAX_RETRIES: &str = "10";
    pub const BLOCK_RANGE: u64 = 10; // Extremely conservative for public RPCs
}

/// Filter mode for the listener
#[derive(Clone)]
pub enum FilterMode {
    /// Filter by specific contract address
    ByAddress(String),
    /// Filter by event topic (for tracking all events of a type)
    ByTopic { topic: String, name: String },
    /// Filter by address AND topic
    ByAddressAndTopic { address: String, topic: String, name: String },
}

pub struct ListenerService {
    pub chain_id: u64,
    pub filter_mode: FilterMode,
    pub db_pool: Pool<Postgres>,
}

impl Service<()> for ListenerService {
    type Response = ();
    type Error = Box<dyn Error + Send + Sync>;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _: ()) -> Self::Future {
        let db_pool = self.db_pool.clone();
        let chain_id = self.chain_id;
        let filter_mode = self.filter_mode.clone();

        Box::pin(async move { fetch_and_save_logs(chain_id, db_pool, filter_mode).await })
    }
}

/// Check if an error is a rate limit error
fn is_rate_limited(err: &alloy::transports::TransportError) -> bool {
    let err_str = err.to_string().to_lowercase();
    err_str.contains("429") 
        || err_str.contains("rate limit") 
        || err_str.contains("too many requests")
        || err_str.contains("-32005")  // BSC "limit exceeded"
        || err_str.contains("limit exceeded")
}

/// Fetch logs with retry logic and exponential backoff
async fn fetch_logs_with_retry<P: Provider>(
    provider: &P,
    filter: &Filter,
    max_retries: u32,
    base_delay_ms: u64,
) -> Result<Vec<Log>, Box<dyn Error + Send + Sync>> {
    for attempt in 0..max_retries {
        match provider.get_logs(filter).await {
            Ok(logs) => {
                // Add delay after successful call to be nice to public RPCs
                sleep(Duration::from_millis(base_delay_ms)).await;
                return Ok(logs);
            }
            Err(e) => {
                if is_rate_limited(&e) {
                    let backoff_ms = base_delay_ms * (2_u64.pow(attempt));
                    eprintln!(
                        "Rate limited (attempt {}/{}), backing off for {}ms",
                        attempt + 1,
                        max_retries,
                        backoff_ms
                    );
                    sleep(Duration::from_millis(backoff_ms)).await;
                } else {
                    // Non-rate-limit error, return immediately
                    return Err(Box::new(e));
                }
            }
        }
    }

    Err(Box::new(AppError::MaxRetriesExceeded(max_retries)))
}

/// Get the sync key for a filter mode (used to track sync progress)
/// Returns a hex string (without 0x prefix) that can be used as an address in the sync log
fn get_sync_key(filter_mode: &FilterMode) -> String {
    match filter_mode {
        FilterMode::ByAddress(addr) => {
            // Strip 0x prefix if present
            addr.strip_prefix("0x").unwrap_or(addr).to_lowercase()
        }
        FilterMode::ByTopic { topic, .. } => {
            // Use first 20 bytes of topic hash as sync key
            topic.strip_prefix("0x").unwrap_or(topic)[..40].to_lowercase()
        }
        FilterMode::ByAddressAndTopic { address, .. } => {
            // Use the contract address as sync key
            address.strip_prefix("0x").unwrap_or(address).to_lowercase()
        }
    }
}

pub async fn fetch_and_save_logs(
    chain_id: u64,
    db_pool: Pool<Postgres>,
    filter_mode: FilterMode,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let rpc_url = env::var("RPC_URL").map_err(|_| AppError::MissingEnvVar("RPC_URL".into()))?;
    
    let rpc_delay_ms = env::var("RPC_DELAY_MS")
        .unwrap_or_else(|_| defaults::RPC_DELAY_MS.to_string())
        .parse::<u64>()
        .unwrap_or(500);
    
    let max_retries = env::var("MAX_RETRIES")
        .unwrap_or_else(|_| defaults::MAX_RETRIES.to_string())
        .parse::<u32>()
        .unwrap_or(3);

    let provider = ProviderBuilder::new().on_builtin(&rpc_url).await?;
    
    let sync_key = get_sync_key(&filter_mode);
    let sync_log = EvmSyncLogs::find_or_create_by_address(&sync_key, chain_id, &db_pool).await?;

    // Fetch latest block with retry
    let latest_block = provider.get_block_number().await?;
    
    if latest_block == sync_log.last_synced_block_number as u64 {
        let display_name = match &filter_mode {
            FilterMode::ByAddress(addr) => addr.clone(),
            FilterMode::ByTopic { name, .. } => name.clone(),
            FilterMode::ByAddressAndTopic { name, .. } => name.clone(),
        };
        println!("Fully indexed: {display_name}");
        return Ok(());
    }

    let from_block_number = match sync_log.last_synced_block_number as u64 {
        0 => {
            // Start from a recent block to avoid massive backfill
            latest_block.saturating_sub(defaults::BLOCK_RANGE)
        }
        block_number => block_number + 1_u64,
    };

    // Conservative block range for public RPCs
    let to_block_number = std::cmp::min(from_block_number + defaults::BLOCK_RANGE, latest_block);

    // Build filter based on mode
    let filter = build_filter(&filter_mode, from_block_number, to_block_number)?;

    // Fetch logs with retry logic
    let logs = fetch_logs_with_retry(&provider, &filter, max_retries, rpc_delay_ms).await?;

    let log_count = logs.len();
    let mut tx = db_pool.begin().await?;
    
    for log in logs {
        let _ = EvmLogs::create(log, &mut *tx)
            .await
            .inspect_err(|error| eprintln!("Error saving log: {error}"));
    }

    let _ = sync_log
        .update_last_synced_block_number(to_block_number, &mut *tx)
        .await
        .inspect_err(|error| eprintln!("Error updating last_synced_block_number: {error}"));

    match tx.commit().await {
        Ok(_) => {
            let display_name = match &filter_mode {
                FilterMode::ByAddress(addr) => addr.clone(),
                FilterMode::ByTopic { name, .. } => name.clone(),
                FilterMode::ByAddressAndTopic { name, .. } => name.clone(),
            };
            println!(
                "Saved {log_count} logs for {display_name}, blocks: {from_block_number} to {to_block_number}"
            );
        }
        Err(err) => eprintln!("Transaction commit error: {err}"),
    }

    Ok(())
}

/// Build a filter based on the filter mode
fn build_filter(
    filter_mode: &FilterMode,
    from_block: u64,
    to_block: u64,
) -> Result<Filter, Box<dyn Error + Send + Sync>> {
    let mut filter = Filter::new()
        .from_block(BlockNumberOrTag::Number(from_block))
        .to_block(BlockNumberOrTag::Number(to_block));

    match filter_mode {
        FilterMode::ByAddress(address) => {
            filter = filter.address(Address::from_str(address)?);
        }
        FilterMode::ByTopic { topic, .. } => {
            let topic_hash = FixedBytes::<32>::from_str(topic)?;
            filter = filter.event_signature(topic_hash);
        }
        FilterMode::ByAddressAndTopic { address, topic, .. } => {
            let topic_hash = FixedBytes::<32>::from_str(topic)?;
            filter = filter
                .address(Address::from_str(address)?)
                .event_signature(topic_hash);
        }
    }

    Ok(filter)
}
