//! PairCreated event handler
//!
//! Handles new token pair creation from PancakeSwap Factory.
//! - Identifies which token is the new memecoin (vs WBNB/BUSD)
//! - Creates token and pair records in database
//! - Creates alert for new token launch

use sqlx::types::BigDecimal;
use std::str::FromStr;

use indexer_db::entity::{
    alert::{AlertEvent, NewAlert, AlertType},
    pair::{NewPair, Pair},
    token::{NewToken, Token},
};

use crate::events::pair_created::PairCreatedEvent;

use super::{HandlerContext, HandlerResult};

/// Process a PairCreated event
///
/// 1. Determine which token is the new memecoin (not WBNB/BUSD)
/// 2. Create a new pair record
/// 3. Create a new token record (or update if exists)
/// 4. Create an alert for the new token launch
pub async fn handle(ctx: &HandlerContext, event: &PairCreatedEvent) -> HandlerResult<()> {
    println!(
        "Processing PairCreated: pair={}, token0={}, token1={}",
        event.pair, event.token0, event.token1
    );

    // Determine which token is the base (WBNB/BUSD) and which is the new token
    let (base_token, new_token, base_index) = if ctx.is_base_token(&event.token0) {
        (&event.token0, &event.token1, 0i16)
    } else if ctx.is_base_token(&event.token1) {
        (&event.token1, &event.token0, 1i16)
    } else {
        // Neither token is a base token - this is a token/token pair, skip for MVP
        println!(
            "Skipping non-base pair: {} / {} (no WBNB/BUSD)",
            event.token0, event.token1
        );
        return Ok(());
    };

    let block_number = event.block.parse::<i64>().unwrap_or(0);

    // Create the pair record
    let new_pair = NewPair {
        address: event.pair.clone(),
        token0_address: event.token0.clone(),
        token1_address: event.token1.clone(),
        factory_address: event.factory.clone(),
        base_token_index: base_index,
        block_number,
    };

    match Pair::create(&new_pair, &ctx.db_pool).await {
        Ok(pair) => {
            println!("Created pair: {} (id={})", pair.address, pair.id);
        }
        Err(e) => {
            // Pair might already exist (idempotent)
            println!("Pair create result: {}", e);
        }
    }

    // Create or update the token record
    let new_token_record = NewToken {
        address: new_token.clone(),
        name: None, // Will be fetched via RPC later
        symbol: None,
        decimals: Some(18), // Default, will be updated
        total_supply: None,
        pair_address: Some(event.pair.clone()),
        creator_address: None, // Would need to trace transaction to get creator
        block_number: Some(block_number),
    };

    match Token::create(&new_token_record, &ctx.db_pool).await {
        Ok(token) => {
            println!(
                "Created token: {} (id={}, pair={})",
                token.address, token.id, event.pair
            );

            // Create alert for new token
            let alert = NewAlert {
                alert_type: AlertType::NewToken.as_str().to_string(),
                token_address: Some(token.address.clone()),
                token_symbol: token.symbol.clone(),
                wallet_address: None,
                title: format!(
                    "New Token: {}",
                    token.symbol.as_deref().unwrap_or(&token.address[..10])
                ),
                message: Some(format!(
                    "New token {} created on PancakeSwap at block {}",
                    token.address, block_number
                )),
                bee_score: None,
                amount_usd: None,
                change_percent: None,
                metadata: None,
            };

            if let Err(e) = AlertEvent::create(&alert, &ctx.db_pool).await {
                eprintln!("Failed to create new token alert: {}", e);
            }
        }
        Err(e) => {
            eprintln!("Failed to create token record: {}", e);
        }
    }

    println!(
        "Processed PairCreated: new_token={}, base={}, pair={}",
        new_token, base_token, event.pair
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base_token_detection() {
        // Test would require mock context
    }
}
