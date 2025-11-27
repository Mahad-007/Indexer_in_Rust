//! Transfer event decoder
//! 
//! Event signature: Transfer(address indexed from, address indexed to, uint256 value)
//! Topic0: 0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef

use indexer_db::entity::evm_logs::EvmLogs;
use serde::Serialize;

use crate::{error::AppError, utils};

/// Decoded Transfer event payload
#[derive(Debug, Serialize)]
pub struct TransferEvent {
    /// Token contract address
    pub token: String,
    /// Sender address
    pub from: String,
    /// Recipient address
    pub to: String,
    /// Transfer amount (hex string to preserve precision for large values)
    pub value: String,
    /// Block number
    pub block: String,
    /// Transaction hash
    pub tx_hash: String,
}

/// Decode a Transfer event from raw log data
/// 
/// Topics layout:
/// - topics[0]: event signature
/// - topics[1]: from (indexed)
/// - topics[2]: to (indexed)
/// 
/// Data layout:
/// - bytes 0-32: value (uint256)
pub fn decode(log: &EvmLogs) -> Result<TransferEvent, AppError> {
    // Ensure we have enough topics
    if log.topics.len() < 3 {
        return Err(AppError::EventDecode(format!(
            "Transfer: expected 3 topics, got {}",
            log.topics.len()
        )));
    }

    // Ensure data is long enough (32 bytes for value)
    if log.data.len() < 32 {
        return Err(AppError::EventDecode(format!(
            "Transfer: expected at least 32 bytes of data, got {}",
            log.data.len()
        )));
    }

    // Token address is the log emitter
    let token = format!("0x{}", utils::vec_to_hex(log.address.to_vec()));

    // Extract from address from topics[1]
    let from = format!("0x{}", utils::vec_to_hex(log.topics[1][12..32].to_vec()));

    // Extract to address from topics[2]
    let to = format!("0x{}", utils::vec_to_hex(log.topics[2][12..32].to_vec()));

    // Extract value from data (as hex string to preserve precision)
    let value = format!("0x{}", utils::vec_to_hex(log.data[0..32].to_vec()));

    let block = log.block_number.to_string();
    let tx_hash = format!("0x{}", utils::vec_to_hex(log.transaction_hash.to_vec()));

    Ok(TransferEvent {
        token,
        from,
        to,
        value,
        block,
        tx_hash,
    })
}

