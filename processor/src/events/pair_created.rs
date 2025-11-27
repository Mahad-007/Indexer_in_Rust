//! PairCreated event decoder
//! 
//! Event signature: PairCreated(address indexed token0, address indexed token1, address pair, uint)
//! Topic0: 0x0d3648bd0f6ba80134a33ba9275ac585d9d315f0ad8355cddefde31afa28d0e9

use indexer_db::entity::evm_logs::EvmLogs;
use serde::Serialize;

use crate::{error::AppError, utils};

/// Decoded PairCreated event payload
#[derive(Debug, Serialize)]
pub struct PairCreatedEvent {
    /// First token address in the pair
    pub token0: String,
    /// Second token address in the pair
    pub token1: String,
    /// Address of the newly created pair contract
    pub pair: String,
    /// Block number where the pair was created
    pub block: String,
    /// Factory address that created the pair
    pub factory: String,
}

/// Decode a PairCreated event from raw log data
/// 
/// Topics layout:
/// - topics[0]: event signature (already matched)
/// - topics[1]: token0 (indexed, 32 bytes, address in last 20 bytes)
/// - topics[2]: token1 (indexed, 32 bytes, address in last 20 bytes)
/// 
/// Data layout:
/// - bytes 0-32: pair address (padded)
/// - bytes 32-64: pair index (uint)
pub fn decode(log: &EvmLogs) -> Result<PairCreatedEvent, AppError> {
    // Ensure we have enough topics
    if log.topics.len() < 3 {
        return Err(AppError::EventDecode(format!(
            "PairCreated: expected 3 topics, got {}",
            log.topics.len()
        )));
    }

    // Extract token0 from topics[1] (last 20 bytes of 32-byte topic)
    let token0 = format!("0x{}", utils::vec_to_hex(log.topics[1][12..32].to_vec()));
    
    // Extract token1 from topics[2] (last 20 bytes of 32-byte topic)
    let token1 = format!("0x{}", utils::vec_to_hex(log.topics[2][12..32].to_vec()));

    // Extract pair address from data (first 32 bytes, address in last 20)
    let pair = if log.data.len() >= 32 {
        format!("0x{}", utils::vec_to_hex(log.data[12..32].to_vec()))
    } else {
        return Err(AppError::EventDecode("PairCreated: data too short for pair address".to_string()));
    };

    // Factory address is the log emitter
    let factory = format!("0x{}", utils::vec_to_hex(log.address.to_vec()));

    // Block number
    let block = log.block_number.to_string();

    Ok(PairCreatedEvent {
        token0,
        token1,
        pair,
        block,
        factory,
    })
}

