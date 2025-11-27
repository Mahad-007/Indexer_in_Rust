//! BeeScore scoring module
//!
//! Calculates the BeeScore (0-100) for tokens based on:
//! - Safety Score (0-60): Liquidity, LP locks, holder distribution, dev holdings, contract safety
//! - Traction Score (0-40): Volume, trades, holder growth, price action, buy/sell balance

pub mod bee_score;

pub use bee_score::{BeeScoreCalculator, BeeScoreResult, ScoreBreakdown};
