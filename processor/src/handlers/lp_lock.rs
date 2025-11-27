//! LP Lock event handler
//!
//! Handles LP lock events from Unicrypt, PinkSale, and Mudra
//! to track liquidity lock status for tokens.

use chrono::{TimeZone, Utc};
use sqlx::types::BigDecimal;
use std::str::FromStr;

use indexer_db::entity::{
    alert::{AlertEvent, AlertType, NewAlert},
    lp_lock::{LpLock, NewLpLock},
    pair::Pair,
    token::Token,
};

use super::{HandlerContext, HandlerResult};

/// Known LP locker contract addresses on BSC
pub mod lockers {
    /// Unicrypt locker
    pub const UNICRYPT: &str = "0xc765bddb93b0d1c1a88282ba0fa6b2d00e3e0c83";
    /// PinkSale locker
    pub const PINKSALE: &str = "0x407993575c91ce7643a4d4ccacc9a98c36ee1bbe";
    /// Mudra locker
    pub const MUDRA: &str = "0xae34bd8a0d1153e51a11a59df23598c304dc5abc";
}

/// LP Lock event decoded structure
#[derive(Debug)]
pub struct LpLockEvent {
    /// LP token (pair) address
    pub lp_token: String,
    /// User who locked
    pub user: String,
    /// Amount locked
    pub amount: String, // hex
    /// Lock timestamp
    pub lock_date: u64,
    /// Unlock timestamp
    pub unlock_date: u64,
    /// Block number
    pub block: String,
    /// Transaction hash
    pub tx_hash: String,
    /// Locker contract address
    pub locker_address: String,
}

/// Parse a hex string to BigDecimal
fn hex_to_bigdecimal(hex: &str) -> BigDecimal {
    let hex_str = hex.trim_start_matches("0x");
    if hex_str.is_empty() || hex_str.chars().all(|c| c == '0') {
        return BigDecimal::from(0);
    }

    match u128::from_str_radix(hex_str, 16) {
        Ok(val) => BigDecimal::from(val),
        Err(_) => BigDecimal::from(0),
    }
}

/// Get locker name from address
fn get_locker_name(address: &str) -> &'static str {
    let addr_lower = address.to_lowercase();
    if addr_lower == lockers::UNICRYPT {
        "unicrypt"
    } else if addr_lower == lockers::PINKSALE {
        "pinksale"
    } else if addr_lower == lockers::MUDRA {
        "mudra"
    } else {
        "unknown"
    }
}

/// Process an LP Lock event
///
/// 1. Look up the pair from LP token address
/// 2. Find the associated token
/// 3. Create LP lock record
/// 4. Update token's lock status
/// 5. Create alert
pub async fn handle(ctx: &HandlerContext, event: &LpLockEvent) -> HandlerResult<()> {
    // Look up the pair (LP token is the pair address)
    let pair = match Pair::find_by_address(&event.lp_token, &ctx.db_pool).await? {
        Some(p) => p,
        None => {
            println!("Unknown LP token for lock: {}", event.lp_token);
            return Ok(());
        }
    };

    // Get the memecoin address from the pair
    let token_address = pair.get_token_address().to_string();

    // Parse amounts and dates
    let locked_amount = hex_to_bigdecimal(&event.amount);
    let lock_date = Utc.timestamp_opt(event.lock_date as i64, 0)
        .single()
        .unwrap_or_else(Utc::now);
    let unlock_date = Utc.timestamp_opt(event.unlock_date as i64, 0)
        .single()
        .unwrap_or_else(Utc::now);
    let block_number = event.block.parse::<i64>().unwrap_or(0);

    // Calculate locked percent (would need total LP supply)
    // For now, use a placeholder - in production, query pair.totalSupply()
    let locked_percent = BigDecimal::from(100); // Placeholder

    let locker_name = get_locker_name(&event.locker_address);

    // Create LP lock record
    let new_lock = NewLpLock {
        token_address: token_address.clone(),
        pair_address: event.lp_token.clone(),
        lock_contract: event.locker_address.clone(),
        lock_contract_name: locker_name.to_string(),
        locked_amount: locked_amount.clone(),
        locked_percent: locked_percent.clone(),
        lock_date,
        unlock_date,
        tx_hash: event.tx_hash.clone(),
        block_number,
    };

    match LpLock::create(&new_lock, &ctx.db_pool).await {
        Ok(lock) => {
            println!(
                "Created LP lock: id={}, token={}, locker={}",
                lock.id, token_address, locker_name
            );
        }
        Err(e) => {
            eprintln!("Failed to create LP lock: {}", e);
        }
    }

    // Update token's LP lock status
    if let Err(e) = Token::update_lp_lock(
        &token_address,
        true,
        &locked_percent,
        Some(unlock_date),
        &ctx.db_pool,
    )
    .await
    {
        eprintln!("Failed to update token LP lock: {}", e);
    }

    // Get token info for alert
    let token = Token::find_by_address(&token_address, &ctx.db_pool).await?;
    let token_symbol = token
        .as_ref()
        .and_then(|t| t.symbol.clone())
        .unwrap_or_else(|| token_address[..10].to_string());

    // Create alert
    let days_locked = (unlock_date - lock_date).num_days();
    let alert = NewAlert {
        alert_type: AlertType::LpLocked.as_str().to_string(),
        token_address: Some(token_address.clone()),
        token_symbol: Some(token_symbol.clone()),
        wallet_address: Some(event.user.clone()),
        title: format!("LP Locked: {} ({} days)", token_symbol, days_locked),
        message: Some(format!(
            "LP tokens for {} locked on {} until {}",
            token_symbol,
            locker_name,
            unlock_date.format("%Y-%m-%d")
        )),
        bee_score: token.as_ref().and_then(|t| t.bee_score),
        amount_usd: None,
        change_percent: Some(locked_percent.clone()),
        metadata: None,
    };

    if let Err(e) = AlertEvent::create(&alert, &ctx.db_pool).await {
        eprintln!("Failed to create LP lock alert: {}", e);
    }

    println!(
        "Processed LP Lock: {} locked for {} days ({})",
        token_symbol, days_locked, locker_name
    );

    Ok(())
}

/// Check if an address is a known LP locker
pub fn is_locker_contract(address: &str) -> bool {
    let addr_lower = address.to_lowercase();
    addr_lower == lockers::UNICRYPT
        || addr_lower == lockers::PINKSALE
        || addr_lower == lockers::MUDRA
}
