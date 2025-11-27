//! Swap event decoder
//! 
//! Event signature: Swap(address indexed sender, uint amount0In, uint amount1In, uint amount0Out, uint amount1Out, address indexed to)
//! Topic0: 0xd78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822

use indexer_db::entity::evm_logs::EvmLogs;
use serde::Serialize;

use crate::{error::AppError, utils};

/// Decoded Swap event payload
#[derive(Debug, Serialize)]
pub struct SwapEvent {
    /// Pair contract address where the swap occurred
    pub pair: String,
    /// Sender address (who initiated the swap)
    pub sender: String,
    /// Amount of token0 swapped in
    pub amount0_in: String,
    /// Amount of token1 swapped in
    pub amount1_in: String,
    /// Amount of token0 received
    pub amount0_out: String,
    /// Amount of token1 received
    pub amount1_out: String,
    /// Recipient address
    pub to: String,
    /// Block number
    pub block: String,
}

/// Decode a Swap event from raw log data
/// 
/// Topics layout:
/// - topics[0]: event signature
/// - topics[1]: sender (indexed)
/// - topics[2]: to (indexed)
/// 
/// Data layout (each 32 bytes):
/// - bytes 0-32: amount0In
/// - bytes 32-64: amount1In
/// - bytes 64-96: amount0Out
/// - bytes 96-128: amount1Out
pub fn decode(log: &EvmLogs) -> Result<SwapEvent, AppError> {
    // Ensure we have enough topics
    if log.topics.len() < 3 {
        return Err(AppError::EventDecode(format!(
            "Swap: expected 3 topics, got {}",
            log.topics.len()
        )));
    }

    // Ensure data is long enough (4 x 32 bytes = 128 bytes)
    if log.data.len() < 128 {
        return Err(AppError::EventDecode(format!(
            "Swap: expected at least 128 bytes of data, got {}",
            log.data.len()
        )));
    }

    // Pair address is the log emitter
    let pair = format!("0x{}", utils::vec_to_hex(log.address.to_vec()));

    // Extract sender from topics[1]
    let sender = format!("0x{}", utils::vec_to_hex(log.topics[1][12..32].to_vec()));

    // Extract to from topics[2]
    let to = format!("0x{}", utils::vec_to_hex(log.topics[2][12..32].to_vec()));

    // Extract amounts from data (as hex strings to preserve precision)
    let amount0_in = format!("0x{}", utils::vec_to_hex(log.data[0..32].to_vec()));
    let amount1_in = format!("0x{}", utils::vec_to_hex(log.data[32..64].to_vec()));
    let amount0_out = format!("0x{}", utils::vec_to_hex(log.data[64..96].to_vec()));
    let amount1_out = format!("0x{}", utils::vec_to_hex(log.data[96..128].to_vec()));

    let block = log.block_number.to_string();

    Ok(SwapEvent {
        pair,
        sender,
        amount0_in,
        amount1_in,
        amount0_out,
        amount1_out,
        to,
        block,
    })
}

