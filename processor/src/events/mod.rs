//! Event decoders for BeanBee BSC indexer
//! 
//! This module contains decoders for the three critical events:
//! - PairCreated: New token launches on PancakeSwap
//! - Swap: Price updates from DEX trades
//! - Transfer: Wallet activity (ERC20 transfers)

pub mod pair_created;
pub mod swap;
pub mod transfer;

use indexer_db::entity::evm_logs::EvmLogs;

use crate::{error::AppError, redis_client::channels, utils};

/// Event topics (keccak256 hashes)
pub mod topics {
    /// PairCreated(address indexed token0, address indexed token1, address pair, uint)
    pub const PAIR_CREATED: &str = "0x0d3648bd0f6ba80134a33ba9275ac585d9d315f0ad8355cddefde31afa28d0e9";
    /// Swap(address indexed sender, uint amount0In, uint amount1In, uint amount0Out, uint amount1Out, address indexed to)
    pub const SWAP: &str = "0xd78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822";
    /// Transfer(address indexed from, address indexed to, uint256 value)
    pub const TRANSFER: &str = "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef";
}

/// Result of decoding an event - contains channel and JSON payload
pub struct DecodedEvent {
    pub channel: &'static str,
    pub payload: String,
}

/// Decode a log into a channel and JSON payload based on its event signature
pub fn decode_event(log: &EvmLogs) -> Result<DecodedEvent, AppError> {
    let topic0 = format!("0x{}", utils::vec_to_hex(log.event_signature.to_vec()));

    match topic0.as_str() {
        topics::PAIR_CREATED => {
            let event = pair_created::decode(log)?;
            Ok(DecodedEvent {
                channel: channels::NEW_PAIR,
                payload: serde_json::to_string(&event)
                    .map_err(|e| AppError::EventDecode(e.to_string()))?,
            })
        }
        topics::SWAP => {
            let event = swap::decode(log)?;
            Ok(DecodedEvent {
                channel: channels::SWAP,
                payload: serde_json::to_string(&event)
                    .map_err(|e| AppError::EventDecode(e.to_string()))?,
            })
        }
        topics::TRANSFER => {
            let event = transfer::decode(log)?;
            Ok(DecodedEvent {
                channel: channels::TRANSFER,
                payload: serde_json::to_string(&event)
                    .map_err(|e| AppError::EventDecode(e.to_string()))?,
            })
        }
        _ => Err(AppError::UnknownEventTopic(topic0)),
    }
}

