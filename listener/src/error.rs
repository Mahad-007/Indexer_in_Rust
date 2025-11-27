use std::fmt::Debug;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Missing `{0}` environment variable")]
    MissingEnvVar(String),

    #[error("Invalid ChainID: `{0}`")]
    InvalidChainID(String),

    #[error("Rate limited by RPC (429), will retry")]
    RateLimited,

    #[error("Max retries ({0}) exceeded")]
    MaxRetriesExceeded(u32),

    #[error("RPC error: {0}")]
    RpcError(String),
}
