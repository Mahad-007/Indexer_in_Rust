//! Event handlers for BeanBee BSC indexer
//!
//! Handlers process decoded events and persist them to the database,
//! including business logic for token tracking, whale detection, etc.

pub mod pair_created;
pub mod swap;
pub mod sync;
pub mod transfer;
pub mod lp_lock;

use alloy::primitives::Address;
use alloy::providers::{Provider, ProviderBuilder};
use alloy::sol;
use sqlx::{Pool, Postgres};
use std::str::FromStr;

use crate::error::AppError;

// Define ERC20 ABI for metadata calls
sol! {
    #[sol(rpc)]
    interface IERC20Metadata {
        function name() external view returns (string);
        function symbol() external view returns (string);
        function decimals() external view returns (uint8);
        function totalSupply() external view returns (uint256);
    }
}

/// Token metadata fetched from blockchain
#[derive(Debug, Clone, Default)]
pub struct TokenMetadata {
    pub name: Option<String>,
    pub symbol: Option<String>,
    pub decimals: Option<i16>,
    pub total_supply: Option<String>,
}

/// Context passed to handlers containing database pool and config
pub struct HandlerContext {
    pub db_pool: Pool<Postgres>,
    pub wbnb_address: String,
    pub busd_address: String,
    pub bnb_price_usd: f64,
    pub whale_threshold_usd: f64,
    pub rpc_url: String,
}

impl HandlerContext {
    pub fn new(
        db_pool: Pool<Postgres>,
        wbnb_address: String,
        busd_address: String,
        bnb_price_usd: f64,
        whale_threshold_usd: f64,
        rpc_url: String,
    ) -> Self {
        Self {
            db_pool,
            wbnb_address,
            busd_address,
            bnb_price_usd,
            whale_threshold_usd,
            rpc_url,
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

    /// Fetch ERC20 token metadata from the blockchain
    pub async fn fetch_token_metadata(&self, token_address: &str) -> TokenMetadata {
        let mut metadata = TokenMetadata::default();

        // Parse the address
        let address = match Address::from_str(token_address) {
            Ok(addr) => addr,
            Err(e) => {
                eprintln!("Invalid token address {}: {}", token_address, e);
                return metadata;
            }
        };

        // Create provider
        let provider = match ProviderBuilder::new().on_http(self.rpc_url.parse().unwrap()) {
            provider => provider,
        };

        // Create contract instance
        let contract = IERC20Metadata::new(address, &provider);

        // Fetch name
        match contract.name().call().await {
            Ok(result) => {
                let name = result._0;
                if !name.is_empty() {
                    metadata.name = Some(name);
                }
            }
            Err(e) => {
                eprintln!("Failed to fetch name for {}: {}", token_address, e);
            }
        }

        // Fetch symbol
        match contract.symbol().call().await {
            Ok(result) => {
                let symbol = result._0;
                if !symbol.is_empty() {
                    metadata.symbol = Some(symbol);
                }
            }
            Err(e) => {
                eprintln!("Failed to fetch symbol for {}: {}", token_address, e);
            }
        }

        // Fetch decimals
        match contract.decimals().call().await {
            Ok(result) => {
                metadata.decimals = Some(result._0 as i16);
            }
            Err(e) => {
                eprintln!("Failed to fetch decimals for {}: {}", token_address, e);
            }
        }

        // Fetch total supply
        match contract.totalSupply().call().await {
            Ok(result) => {
                metadata.total_supply = Some(result._0.to_string());
            }
            Err(e) => {
                eprintln!("Failed to fetch totalSupply for {}: {}", token_address, e);
            }
        }

        println!(
            "Fetched metadata for {}: name={:?}, symbol={:?}, decimals={:?}",
            token_address, metadata.name, metadata.symbol, metadata.decimals
        );

        metadata
    }
}

/// Result type for handlers
pub type HandlerResult<T> = Result<T, AppError>;
