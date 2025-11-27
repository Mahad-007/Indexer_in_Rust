//! Transfer event handler
//!
//! Handles ERC20 Transfer events to:
//! - Track holder balances
//! - Identify snipers (early buyers)
//! - Track dev wallet movements
//! - Create wallet activity records

use chrono::Utc;
use sqlx::types::BigDecimal;
use std::str::FromStr;

use indexer_db::entity::{
    alert::{AlertEvent, AlertType, NewAlert},
    token::Token,
    token_holder::{NewTokenHolder, TokenHolder},
    wallet_activity::{NewWalletActivity, WalletActivity},
};

use crate::events::transfer::TransferEvent;

use super::{HandlerContext, HandlerResult};

/// Parse a hex string (0x...) to BigDecimal
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

/// Zero address constant
const ZERO_ADDRESS: &str = "0x0000000000000000000000000000000000000000";

/// Dead address (burn)
const DEAD_ADDRESS: &str = "0x000000000000000000000000000000000000dead";

/// Process a Transfer event
///
/// 1. Update sender's balance (decrease)
/// 2. Update recipient's balance (increase)
/// 3. Check for sniper activity (early blocks)
/// 4. Check for dev sells
/// 5. Create wallet activity records
pub async fn handle(ctx: &HandlerContext, event: &TransferEvent) -> HandlerResult<()> {
    let token_address = event.token.clone();
    let from_address = event.from.clone();
    let to_address = event.to.clone();
    let value = hex_to_bigdecimal(&event.value);

    // Skip zero-value transfers
    if value == BigDecimal::from(0) {
        return Ok(());
    }

    // Check if this token is being tracked
    let token = match Token::find_by_address(&token_address, &ctx.db_pool).await? {
        Some(t) => t,
        None => {
            // Token not in our database, skip
            return Ok(());
        }
    };

    let block_number = event.block.parse::<i64>().unwrap_or(0);
    let token_creation_block = token.block_number.unwrap_or(0);
    let token_symbol = token.symbol.clone().unwrap_or_else(|| token_address[..10].to_string());

    // Determine if this is a mint (from zero address)
    let is_mint = from_address.to_lowercase() == ZERO_ADDRESS;

    // Determine if this is a burn (to zero or dead address)
    let is_burn = to_address.to_lowercase() == ZERO_ADDRESS
        || to_address.to_lowercase() == DEAD_ADDRESS;

    // Check if sender is a sniper (bought in first 2 blocks)
    let is_from_sniper = if !is_mint {
        match TokenHolder::find_sniper_holders(&token_address, &ctx.db_pool).await {
            Ok(snipers) => snipers.iter().any(|s| s.wallet_address.to_lowercase() == from_address.to_lowercase()),
            Err(_) => false,
        }
    } else {
        false
    };

    // Check if sender is a dev
    let is_from_dev = if !is_mint {
        match TokenHolder::find_dev_holders(&token_address, &ctx.db_pool).await {
            Ok(devs) => devs.iter().any(|d| d.wallet_address.to_lowercase() == from_address.to_lowercase()),
            Err(_) => false,
        }
    } else {
        false
    };

    // Update sender's balance (if not mint)
    if !is_mint {
        // For simplicity, we're just recording activity
        // Full balance tracking would require RPC calls to get current balance
        let activity = NewWalletActivity {
            wallet_address: from_address.clone(),
            tx_hash: event.tx_hash.clone(),
            block_number,
            timestamp: Utc::now(),
            action: "transfer_out".to_string(),
            token_address: token_address.clone(),
            token_symbol: Some(token_symbol.clone()),
            amount_tokens: Some(value.clone()),
            amount_usd: None, // Would need price lookup
        };

        if let Err(e) = WalletActivity::create(&activity, &ctx.db_pool).await {
            // Might be duplicate
            println!("Wallet activity (from) result: {}", e);
        }
    }

    // Update recipient's balance (if not burn)
    if !is_burn {
        // Determine if recipient is a sniper (receiving in first 2 blocks after token creation)
        let is_sniper = block_number <= token_creation_block + 2 && !is_mint;

        let holder = NewTokenHolder {
            token_address: token_address.clone(),
            wallet_address: to_address.clone(),
            balance: value.clone(), // This should be cumulative, simplified here
            is_dev: false,
            is_sniper,
            is_contract: false, // Would need to check via RPC
            first_buy_block: Some(block_number),
        };

        if let Err(e) = TokenHolder::upsert(&holder, &ctx.db_pool).await {
            eprintln!("Failed to upsert token holder: {}", e);
        }

        // Mark as sniper if applicable
        if is_sniper {
            if let Err(e) = TokenHolder::mark_as_sniper(&token_address, &to_address, &ctx.db_pool).await {
                eprintln!("Failed to mark sniper: {}", e);
            }
        }

        // Create wallet activity for recipient
        let activity = NewWalletActivity {
            wallet_address: to_address.clone(),
            tx_hash: event.tx_hash.clone(),
            block_number,
            timestamp: Utc::now(),
            action: "transfer_in".to_string(),
            token_address: token_address.clone(),
            token_symbol: Some(token_symbol.clone()),
            amount_tokens: Some(value.clone()),
            amount_usd: None,
        };

        if let Err(e) = WalletActivity::create(&activity, &ctx.db_pool).await {
            println!("Wallet activity (to) result: {}", e);
        }
    }

    // Create alert for dev sell
    if is_from_dev && !is_burn {
        let alert = NewAlert {
            alert_type: AlertType::DevSell.as_str().to_string(),
            token_address: Some(token_address.clone()),
            token_symbol: Some(token_symbol.clone()),
            wallet_address: Some(from_address.clone()),
            title: format!("Dev Sell: {}", token_symbol),
            message: Some(format!(
                "Developer wallet transferred {} tokens at block {}",
                value, block_number
            )),
            bee_score: token.bee_score,
            amount_usd: None,
            change_percent: None,
            metadata: None,
        };

        if let Err(e) = AlertEvent::create(&alert, &ctx.db_pool).await {
            eprintln!("Failed to create dev sell alert: {}", e);
        }
    }

    println!(
        "Processed Transfer: {} -> {} ({} tokens of {})",
        if is_mint { "MINT" } else { &from_address[..10] },
        if is_burn { "BURN" } else { &to_address[..10] },
        value,
        token_symbol
    );

    Ok(())
}
