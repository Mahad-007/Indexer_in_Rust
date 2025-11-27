//! Sync event handler
//!
//! Handles Sync events from DEX pairs to update:
//! - Reserve amounts
//! - Liquidity calculations
//! - Price snapshots

use chrono::Utc;
use sqlx::types::BigDecimal;
use std::str::FromStr;

use indexer_db::entity::{
    pair::Pair,
    price_snapshot::{NewPriceSnapshot, PriceSnapshot},
    token::Token,
};

use super::{HandlerContext, HandlerResult};

/// Decoded Sync event (would come from event decoder)
#[derive(Debug)]
pub struct SyncEvent {
    pub pair: String,
    pub reserve0: String, // hex
    pub reserve1: String, // hex
    pub block: String,
}

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

/// Convert token amount to human-readable format
fn to_decimal_amount(raw: &BigDecimal, decimals: u8) -> f64 {
    let divisor = 10u128.pow(decimals as u32) as f64;
    raw.to_string().parse::<f64>().unwrap_or(0.0) / divisor
}

/// Process a Sync event
///
/// 1. Look up the pair
/// 2. Update reserves
/// 3. Calculate liquidity in USD
/// 4. Update token price and liquidity
/// 5. Create price snapshot (throttled)
pub async fn handle(ctx: &HandlerContext, event: &SyncEvent) -> HandlerResult<()> {
    // Look up the pair
    let pair = match Pair::find_by_address(&event.pair, &ctx.db_pool).await? {
        Some(p) => p,
        None => {
            println!("Unknown pair for Sync: {}", event.pair);
            return Ok(());
        }
    };

    // Parse reserves
    let reserve0 = hex_to_bigdecimal(&event.reserve0);
    let reserve1 = hex_to_bigdecimal(&event.reserve1);

    // Update pair reserves
    if let Err(e) = Pair::update_reserves(&event.pair, &reserve0, &reserve1, &ctx.db_pool).await {
        eprintln!("Failed to update pair reserves: {}", e);
    }

    // Determine which reserve is BNB and which is the token
    let (bnb_reserve, token_reserve, token_address) = match pair.base_token_index {
        Some(0) => {
            // token0 is WBNB
            (reserve0.clone(), reserve1.clone(), pair.token1_address.clone())
        }
        Some(1) => {
            // token1 is WBNB
            (reserve1.clone(), reserve0.clone(), pair.token0_address.clone())
        }
        _ => {
            println!("Unknown base token index for pair {}", event.pair);
            return Ok(());
        }
    };

    // Calculate liquidity (2 * BNB reserve * BNB price)
    let bnb_reserve_decimal = to_decimal_amount(&bnb_reserve, 18);
    let liquidity_usd = 2.0 * bnb_reserve_decimal * ctx.bnb_price_usd;
    let liquidity_bnb = 2.0 * bnb_reserve_decimal;

    // Calculate token price from reserves
    // price_in_bnb = bnb_reserve / token_reserve
    let token_reserve_decimal = to_decimal_amount(&token_reserve, 18);
    let price_bnb = if token_reserve_decimal > 0.0 {
        bnb_reserve_decimal / token_reserve_decimal
    } else {
        0.0
    };
    let price_usd = price_bnb * ctx.bnb_price_usd;

    // Update token price and liquidity
    let price_usd_bd = BigDecimal::from_str(&format!("{:.18}", price_usd)).unwrap_or(BigDecimal::from(0));
    let price_bnb_bd = BigDecimal::from_str(&format!("{:.18}", price_bnb)).unwrap_or(BigDecimal::from(0));
    let liquidity_usd_bd = BigDecimal::from_str(&format!("{:.2}", liquidity_usd)).unwrap_or(BigDecimal::from(0));
    let liquidity_bnb_bd = BigDecimal::from_str(&format!("{:.18}", liquidity_bnb)).unwrap_or(BigDecimal::from(0));

    if let Err(e) = Token::update_price_metrics(
        &token_address,
        &price_usd_bd,
        &price_bnb_bd,
        &liquidity_usd_bd,
        &liquidity_bnb_bd,
        &ctx.db_pool,
    )
    .await
    {
        eprintln!("Failed to update token price metrics: {}", e);
    }

    // Create price snapshot
    // In production, throttle this to every 5 minutes to avoid too many records
    let now = Utc::now();

    // Get holder count from token (would need separate tracking)
    let holder_count = match Token::find_by_address(&token_address, &ctx.db_pool).await {
        Ok(Some(t)) => t.holder_count,
        _ => None,
    };

    // Calculate market cap (price * total supply)
    // For now, we don't have total supply, so skip market cap
    let market_cap_usd: Option<BigDecimal> = None;

    let snapshot = NewPriceSnapshot {
        token_address: token_address.clone(),
        timestamp: now,
        price_usd: Some(price_usd_bd.clone()),
        price_bnb: Some(price_bnb_bd.clone()),
        liquidity_usd: Some(liquidity_usd_bd.clone()),
        volume_usd: None, // Would need to aggregate from swaps
        market_cap_usd,
        holder_count,
    };

    if let Err(e) = PriceSnapshot::create(&snapshot, &ctx.db_pool).await {
        // Might be duplicate timestamp
        println!("Price snapshot result: {}", e);
    }

    println!(
        "Processed Sync: {} - price=${:.10}, liquidity=${:.2}",
        token_address, price_usd, liquidity_usd
    );

    Ok(())
}
