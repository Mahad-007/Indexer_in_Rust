//! Swap event handler
//!
//! Handles swap events from DEX pairs to:
//! - Track price, volume, and trade metrics
//! - Detect whale transactions
//! - Update token statistics

use chrono::Utc;
use sqlx::types::BigDecimal;
use std::str::FromStr;

use indexer_db::entity::{
    alert::{AlertEvent, AlertType, NewAlert},
    pair::Pair,
    swap::{NewSwap, Swap},
    token::Token,
};

use crate::events::swap::SwapEvent;

use super::{HandlerContext, HandlerResult};

/// Parse a hex string (0x...) to BigDecimal
fn hex_to_bigdecimal(hex: &str) -> BigDecimal {
    let hex_str = hex.trim_start_matches("0x");
    if hex_str.is_empty() || hex_str.chars().all(|c| c == '0') {
        return BigDecimal::from(0);
    }

    // Parse as u128 for reasonable precision, convert to BigDecimal
    match u128::from_str_radix(hex_str, 16) {
        Ok(val) => BigDecimal::from(val),
        Err(_) => {
            // For very large numbers, try to handle gracefully
            BigDecimal::from(0)
        }
    }
}

/// Convert token amount to human-readable format (divide by 10^decimals)
fn to_decimal_amount(raw: &BigDecimal, decimals: u8) -> f64 {
    let divisor = 10u128.pow(decimals as u32) as f64;
    raw.to_string().parse::<f64>().unwrap_or(0.0) / divisor
}

/// Process a Swap event
///
/// 1. Look up the pair to identify tokens
/// 2. Determine trade direction (buy/sell based on WBNB flow)
/// 3. Calculate USD value
/// 4. Create swap record
/// 5. Update token metrics
/// 6. Check for whale transaction
pub async fn handle(ctx: &HandlerContext, event: &SwapEvent) -> HandlerResult<()> {
    // Look up the pair
    let pair = match Pair::find_by_address(&event.pair, &ctx.db_pool).await? {
        Some(p) => p,
        None => {
            // Pair not in our database - might be from before we started indexing
            println!("Unknown pair: {}, skipping swap", event.pair);
            return Ok(());
        }
    };

    // Get the non-base token address (the memecoin)
    let token_address = pair.get_token_address().to_string();
    let base_address = pair.get_base_address().to_string();

    // Parse amounts
    let amount0_in = hex_to_bigdecimal(&event.amount0_in);
    let amount1_in = hex_to_bigdecimal(&event.amount1_in);
    let amount0_out = hex_to_bigdecimal(&event.amount0_out);
    let amount1_out = hex_to_bigdecimal(&event.amount1_out);

    // Determine trade direction and amounts
    // Buy: BNB in, tokens out
    // Sell: tokens in, BNB out
    let (is_buy, amount_tokens, amount_bnb) = match pair.base_token_index {
        Some(0) => {
            // token0 is base (WBNB), token1 is memecoin
            if amount0_in > BigDecimal::from(0) && amount1_out > BigDecimal::from(0) {
                // BNB in -> tokens out = BUY
                (true, amount1_out.clone(), amount0_in.clone())
            } else if amount1_in > BigDecimal::from(0) && amount0_out > BigDecimal::from(0) {
                // tokens in -> BNB out = SELL
                (false, amount1_in.clone(), amount0_out.clone())
            } else {
                println!("Ambiguous swap direction, skipping");
                return Ok(());
            }
        }
        Some(1) => {
            // token1 is base (WBNB), token0 is memecoin
            if amount1_in > BigDecimal::from(0) && amount0_out > BigDecimal::from(0) {
                // BNB in -> tokens out = BUY
                (true, amount0_out.clone(), amount1_in.clone())
            } else if amount0_in > BigDecimal::from(0) && amount1_out > BigDecimal::from(0) {
                // tokens in -> BNB out = SELL
                (false, amount0_in.clone(), amount1_out.clone())
            } else {
                println!("Ambiguous swap direction, skipping");
                return Ok(());
            }
        }
        _ => {
            println!("Unknown base token index for pair {}", event.pair);
            return Ok(());
        }
    };

    // Calculate USD value (BNB amount * BNB price)
    let bnb_amount_decimal = to_decimal_amount(&amount_bnb, 18);
    let amount_usd = bnb_amount_decimal * ctx.bnb_price_usd;
    let amount_usd_bd = BigDecimal::from_str(&format!("{:.2}", amount_usd)).unwrap_or(BigDecimal::from(0));

    // Check if whale trade
    let is_whale = amount_usd >= ctx.whale_threshold_usd;

    let block_number = event.block.parse::<i64>().unwrap_or(0);
    let trade_type = if is_buy { "buy" } else { "sell" };

    // Calculate price (USD per token)
    let tokens_decimal = to_decimal_amount(&amount_tokens, 18);
    let price_usd = if tokens_decimal > 0.0 {
        amount_usd / tokens_decimal
    } else {
        0.0
    };
    let price_usd_bd = BigDecimal::from_str(&format!("{:.18}", price_usd)).unwrap_or(BigDecimal::from(0));

    // Create swap record
    let new_swap = NewSwap {
        tx_hash: format!("0x{}", "0".repeat(64)), // We don't have tx_hash from the event struct, would need from log
        block_number,
        log_index: 0, // Would need from log
        timestamp: Utc::now(),
        pair_address: event.pair.clone(),
        token_address: token_address.clone(),
        wallet_address: event.to.clone(), // Recipient is the trader
        trade_type: trade_type.to_string(),
        amount_tokens: Some(amount_tokens.clone()),
        amount_bnb: Some(amount_bnb.clone()),
        amount_usd: Some(amount_usd_bd.clone()),
        price_usd: Some(price_usd_bd.clone()),
        is_whale,
    };

    match Swap::create(&new_swap, &ctx.db_pool).await {
        Ok(swap) => {
            println!(
                "Created swap: {} {} ${:.2} of {} (whale={})",
                trade_type.to_uppercase(),
                swap.id,
                amount_usd,
                token_address,
                is_whale
            );
        }
        Err(e) => {
            // Might be duplicate (idempotent)
            println!("Swap create result: {}", e);
        }
    }

    // Update token metrics
    if let Err(e) = Token::increment_trade_count(
        &token_address,
        is_buy,
        &amount_usd_bd,
        &ctx.db_pool,
    )
    .await
    {
        eprintln!("Failed to update token trade count: {}", e);
    }

    // Update token price
    let price_bnb = if tokens_decimal > 0.0 {
        bnb_amount_decimal / tokens_decimal
    } else {
        0.0
    };
    let price_bnb_bd = BigDecimal::from_str(&format!("{:.18}", price_bnb)).unwrap_or(BigDecimal::from(0));

    // We'd need reserves to calculate liquidity properly - for now skip
    // This would come from Sync events

    // Create whale alert if applicable
    if is_whale {
        // Try to get token symbol
        let token_symbol = match Token::find_by_address(&token_address, &ctx.db_pool).await {
            Ok(Some(t)) => t.symbol.unwrap_or_else(|| token_address[..10].to_string()),
            _ => token_address[..10].to_string(),
        };

        let alert = NewAlert {
            alert_type: if is_buy {
                AlertType::WhaleBuy.as_str().to_string()
            } else {
                AlertType::WhaleSell.as_str().to_string()
            },
            token_address: Some(token_address.clone()),
            token_symbol: Some(token_symbol.clone()),
            wallet_address: Some(event.to.clone()),
            title: format!(
                "Whale {}: ${:.0} {}",
                if is_buy { "Buy" } else { "Sell" },
                amount_usd,
                token_symbol
            ),
            message: Some(format!(
                "Whale {} ${:.2} worth of {} at block {}",
                if is_buy { "bought" } else { "sold" },
                amount_usd,
                token_symbol,
                block_number
            )),
            bee_score: None,
            amount_usd: Some(amount_usd_bd),
            change_percent: None,
            metadata: None,
        };

        if let Err(e) = AlertEvent::create(&alert, &ctx.db_pool).await {
            eprintln!("Failed to create whale alert: {}", e);
        }
    }

    println!(
        "Processed Swap: {} {} ${:.2} of {} (price=${:.10})",
        trade_type.to_uppercase(),
        if is_whale { "[WHALE]" } else { "" },
        amount_usd,
        token_address,
        price_usd
    );

    Ok(())
}
