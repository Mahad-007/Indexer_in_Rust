//! BeeScore Calculator
//!
//! Calculates a score from 0-100 for each token to help users
//! identify promising vs risky tokens.
//!
//! Score Components:
//! - Safety Score (0-60): How safe is this token?
//! - Traction Score (0-40): How much momentum does it have?

use indexer_db::entity::token::TokenMetrics;

/// Result of BeeScore calculation
#[derive(Debug, Clone)]
pub struct BeeScoreResult {
    /// Total score (0-100)
    pub total: u8,
    /// Safety score component (0-60)
    pub safety_score: u8,
    /// Safety score breakdown
    pub safety_breakdown: Vec<ScoreBreakdown>,
    /// Traction score component (0-40)
    pub traction_score: u8,
    /// Traction score breakdown
    pub traction_breakdown: Vec<ScoreBreakdown>,
}

/// Individual score component breakdown
#[derive(Debug, Clone)]
pub struct ScoreBreakdown {
    pub name: String,
    pub score: u8,
    pub max_score: u8,
    pub reason: String,
}

/// BeeScore calculator
pub struct BeeScoreCalculator;

impl BeeScoreCalculator {
    /// Calculate BeeScore (0-100) for a token
    /// Safety (0-60) + Traction (0-40) = Total Score
    pub fn calculate(metrics: &TokenMetrics) -> BeeScoreResult {
        let (safety_score, safety_breakdown) = Self::calculate_safety(metrics);
        let (traction_score, traction_breakdown) = Self::calculate_traction(metrics);

        BeeScoreResult {
            total: safety_score + traction_score,
            safety_score,
            safety_breakdown,
            traction_score,
            traction_breakdown,
        }
    }

    /// Calculate Safety Score (0-60)
    ///
    /// Components:
    /// - Liquidity (0-15): Higher liquidity = safer
    /// - LP Locked (0-15): Locked liquidity prevents rugs
    /// - Holder Distribution (0-15): Decentralized = safer
    /// - Dev Holdings (0-10): Lower dev holdings = safer
    /// - Contract Safety (0-5): Renounced ownership = safer
    fn calculate_safety(metrics: &TokenMetrics) -> (u8, Vec<ScoreBreakdown>) {
        let mut score: u8 = 0;
        let mut breakdown = Vec::new();

        // Liquidity (0-15 points)
        // < $10k = 0, $10-50k = 5, $50-100k = 10, > $100k = 15
        let (liq_score, liq_reason) = match metrics.liquidity_usd {
            l if l >= 100_000.0 => (15, "Excellent liquidity (>$100k)"),
            l if l >= 50_000.0 => (10, "Good liquidity ($50k-$100k)"),
            l if l >= 10_000.0 => (5, "Low liquidity ($10k-$50k)"),
            _ => (0, "Very low liquidity (<$10k)"),
        };
        score += liq_score;
        breakdown.push(ScoreBreakdown {
            name: "Liquidity".to_string(),
            score: liq_score,
            max_score: 15,
            reason: liq_reason.to_string(),
        });

        // LP Locked (0-15 points)
        // Not locked = 0, < 50% locked = 5, 50-90% = 10, > 90% = 15
        let (lock_score, lock_reason) = if !metrics.lp_locked {
            (0, "LP not locked - high rug risk")
        } else {
            match metrics.lp_lock_percent {
                p if p >= 90.0 => (15, "LP >90% locked - excellent"),
                p if p >= 50.0 => (10, "LP 50-90% locked - good"),
                _ => (5, "LP <50% locked - moderate risk"),
            }
        };
        score += lock_score;
        breakdown.push(ScoreBreakdown {
            name: "LP Lock".to_string(),
            score: lock_score,
            max_score: 15,
            reason: lock_reason.to_string(),
        });

        // Holder Distribution (0-15 points)
        // Top 10 > 80% = 0, 60-80% = 5, 40-60% = 10, < 40% = 15
        let (dist_score, dist_reason) = match metrics.top_10_holder_percent {
            p if p < 40.0 => (15, "Well distributed (<40% top 10)"),
            p if p < 60.0 => (10, "Moderately distributed (40-60% top 10)"),
            p if p < 80.0 => (5, "Concentrated (60-80% top 10)"),
            _ => (0, "Highly concentrated (>80% top 10)"),
        };
        score += dist_score;
        breakdown.push(ScoreBreakdown {
            name: "Distribution".to_string(),
            score: dist_score,
            max_score: 15,
            reason: dist_reason.to_string(),
        });

        // Dev Holdings (0-10 points)
        // > 20% = 0, 10-20% = 3, 5-10% = 7, < 5% = 10
        let (dev_score, dev_reason) = match metrics.dev_holdings_percent {
            p if p < 5.0 => (10, "Low dev holdings (<5%)"),
            p if p < 10.0 => (7, "Moderate dev holdings (5-10%)"),
            p if p < 20.0 => (3, "High dev holdings (10-20%)"),
            _ => (0, "Very high dev holdings (>20%)"),
        };
        score += dev_score;
        breakdown.push(ScoreBreakdown {
            name: "Dev Holdings".to_string(),
            score: dev_score,
            max_score: 10,
            reason: dev_reason.to_string(),
        });

        // Contract Safety (0-5 points)
        // Ownership renounced = +5
        let (contract_score, contract_reason) = if metrics.ownership_renounced {
            (5, "Ownership renounced")
        } else {
            (0, "Ownership not renounced")
        };
        score += contract_score;
        breakdown.push(ScoreBreakdown {
            name: "Contract".to_string(),
            score: contract_score,
            max_score: 5,
            reason: contract_reason.to_string(),
        });

        (score, breakdown)
    }

    /// Calculate Traction Score (0-40)
    ///
    /// Components:
    /// - Volume (0-12): Healthy trading volume relative to liquidity
    /// - Trade Count (0-8): Active trading indicates interest
    /// - Holder Growth (0-8): Growing holder count is bullish
    /// - Price Action (0-6): Healthy gains, not extreme pumps/dumps
    /// - Buy/Sell Balance (0-6): Balanced trading with slight buy pressure
    fn calculate_traction(metrics: &TokenMetrics) -> (u8, Vec<ScoreBreakdown>) {
        let mut score: u8 = 0;
        let mut breakdown = Vec::new();

        // Volume (0-12 points)
        // Based on volume relative to liquidity (healthy = 50-200%)
        let vol_ratio = if metrics.liquidity_usd > 0.0 {
            metrics.volume_1h_usd / metrics.liquidity_usd
        } else {
            0.0
        };
        let (vol_score, vol_reason) = match vol_ratio {
            r if r >= 0.5 && r <= 2.0 => (12, "Healthy volume (50-200% of liquidity)"),
            r if r >= 0.2 && r <= 3.0 => (8, "Good volume (20-300% of liquidity)"),
            r if r >= 0.1 => (4, "Low volume (>10% of liquidity)"),
            _ => (0, "Very low volume"),
        };
        score += vol_score;
        breakdown.push(ScoreBreakdown {
            name: "Volume".to_string(),
            score: vol_score,
            max_score: 12,
            reason: vol_reason.to_string(),
        });

        // Trade Count (0-8 points)
        let (trades_score, trades_reason) = match metrics.trades_1h {
            t if t >= 100 => (8, "Very active (100+ trades/hr)"),
            t if t >= 50 => (6, "Active (50-100 trades/hr)"),
            t if t >= 20 => (4, "Moderate activity (20-50 trades/hr)"),
            t if t >= 5 => (2, "Low activity (5-20 trades/hr)"),
            _ => (0, "Very low activity (<5 trades/hr)"),
        };
        score += trades_score;
        breakdown.push(ScoreBreakdown {
            name: "Trades".to_string(),
            score: trades_score,
            max_score: 8,
            reason: trades_reason.to_string(),
        });

        // Holder Growth (0-8 points)
        let growth = if metrics.holder_count_1h_ago > 0 {
            ((metrics.holder_count - metrics.holder_count_1h_ago) as f64
                / metrics.holder_count_1h_ago as f64)
                * 100.0
        } else {
            0.0
        };
        let (growth_score, growth_reason) = match growth {
            g if g >= 20.0 => (8, "Strong growth (20%+ new holders/hr)"),
            g if g >= 10.0 => (6, "Good growth (10-20% new holders/hr)"),
            g if g >= 5.0 => (4, "Moderate growth (5-10% new holders/hr)"),
            g if g > 0.0 => (2, "Slight growth (<5% new holders/hr)"),
            _ => (0, "No holder growth"),
        };
        score += growth_score;
        breakdown.push(ScoreBreakdown {
            name: "Growth".to_string(),
            score: growth_score,
            max_score: 8,
            reason: growth_reason.to_string(),
        });

        // Price Action (0-6 points)
        // Healthy = moderate gains (5-100%), not extreme pumps or dumps
        let (price_score, price_reason) = match metrics.price_change_1h {
            p if p >= 5.0 && p <= 100.0 => (6, "Healthy gain (5-100%)"),
            p if p >= 0.0 && p <= 200.0 => (4, "Acceptable price action (0-200%)"),
            p if p >= -20.0 => (2, "Small dip (<20% loss)"),
            p if p < -50.0 => (0, "Major dump (>50% loss)"),
            _ => (1, "Volatile price action"),
        };
        score += price_score;
        breakdown.push(ScoreBreakdown {
            name: "Price Action".to_string(),
            score: price_score,
            max_score: 6,
            reason: price_reason.to_string(),
        });

        // Buy/Sell Balance (0-6 points)
        let total_trades = (metrics.buys_1h + metrics.sells_1h) as f64;
        let buy_ratio = if total_trades > 0.0 {
            metrics.buys_1h as f64 / total_trades
        } else {
            0.5
        };
        let (balance_score, balance_reason) = match buy_ratio {
            r if r >= 0.4 && r <= 0.7 => (6, "Balanced with buy pressure (40-70% buys)"),
            r if r >= 0.3 && r <= 0.8 => (4, "Acceptable balance (30-80% buys)"),
            r if r >= 0.2 => (2, "Sell pressure (only 20-30% buys)"),
            _ => (0, "Heavy selling (<20% buys)"),
        };
        score += balance_score;
        breakdown.push(ScoreBreakdown {
            name: "Buy/Sell".to_string(),
            score: balance_score,
            max_score: 6,
            reason: balance_reason.to_string(),
        });

        (score, breakdown)
    }

    /// Get a human-readable rating based on score
    pub fn get_rating(score: u8) -> &'static str {
        match score {
            80..=100 => "Excellent",
            60..=79 => "Good",
            40..=59 => "Fair",
            20..=39 => "Poor",
            _ => "Risky",
        }
    }

    /// Get rating color (for UI)
    pub fn get_rating_color(score: u8) -> &'static str {
        match score {
            80..=100 => "green",
            60..=79 => "lime",
            40..=59 => "yellow",
            20..=39 => "orange",
            _ => "red",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_perfect_token() {
        let metrics = TokenMetrics {
            liquidity_usd: 150_000.0,
            lp_locked: true,
            lp_lock_percent: 95.0,
            top_10_holder_percent: 30.0,
            dev_holdings_percent: 3.0,
            ownership_renounced: true,
            volume_1h_usd: 100_000.0, // ~67% of liquidity
            trades_1h: 150,
            holder_count: 500,
            holder_count_1h_ago: 400, // 25% growth
            price_change_1h: 50.0,
            buys_1h: 100,
            sells_1h: 50, // 67% buys
        };

        let result = BeeScoreCalculator::calculate(&metrics);

        assert_eq!(result.safety_score, 60); // Max safety
        assert_eq!(result.traction_score, 40); // Max traction
        assert_eq!(result.total, 100);
    }

    #[test]
    fn test_risky_token() {
        let metrics = TokenMetrics {
            liquidity_usd: 5_000.0,
            lp_locked: false,
            lp_lock_percent: 0.0,
            top_10_holder_percent: 90.0,
            dev_holdings_percent: 30.0,
            ownership_renounced: false,
            volume_1h_usd: 100.0,
            trades_1h: 2,
            holder_count: 10,
            holder_count_1h_ago: 10,
            price_change_1h: -60.0,
            buys_1h: 1,
            sells_1h: 9,
        };

        let result = BeeScoreCalculator::calculate(&metrics);

        assert_eq!(result.safety_score, 0);
        assert_eq!(result.traction_score, 0);
        assert_eq!(result.total, 0);
    }

    #[test]
    fn test_rating() {
        assert_eq!(BeeScoreCalculator::get_rating(85), "Excellent");
        assert_eq!(BeeScoreCalculator::get_rating(65), "Good");
        assert_eq!(BeeScoreCalculator::get_rating(45), "Fair");
        assert_eq!(BeeScoreCalculator::get_rating(25), "Poor");
        assert_eq!(BeeScoreCalculator::get_rating(10), "Risky");
    }
}
