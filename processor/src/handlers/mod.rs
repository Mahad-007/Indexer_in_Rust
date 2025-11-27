//! Event handlers for BeanBee BSC indexer
//!
//! Handlers process decoded events and persist them to the database,
//! including business logic for token tracking, whale detection, etc.

pub mod pair_created;
pub mod swap;
pub mod sync;
pub mod transfer;
pub mod lp_lock;

use sqlx::{Pool, Postgres};

use crate::error::AppError;

/// Context passed to handlers containing database pool and config
pub struct HandlerContext {
    pub db_pool: Pool<Postgres>,
    pub wbnb_address: String,
    pub busd_address: String,
    pub bnb_price_usd: f64,
    pub whale_threshold_usd: f64,
}

impl HandlerContext {
    pub fn new(
        db_pool: Pool<Postgres>,
        wbnb_address: String,
        busd_address: String,
        bnb_price_usd: f64,
        whale_threshold_usd: f64,
    ) -> Self {
        Self {
            db_pool,
            wbnb_address,
            busd_address,
            bnb_price_usd,
            whale_threshold_usd,
        }
    }

    /// Check if address is WBNB
    pub fn is_wbnb(&self, address: &str) -> bool {
        address.to_lowercase() == self.wbnb_address.to_lowercase()
    }

    /// Check if address is BUSD
    pub fn is_busd(&self, address: &str) -> bool {
        address.to_lowercase() == self.busd_address.to_lowercase()
    }

    /// Check if address is a base token (WBNB or BUSD)
    pub fn is_base_token(&self, address: &str) -> bool {
        self.is_wbnb(address) || self.is_busd(address)
    }
}

/// Result type for handlers
pub type HandlerResult<T> = Result<T, AppError>;
