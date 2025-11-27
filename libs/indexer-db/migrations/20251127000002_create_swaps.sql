-- Swaps/Trades table for tracking all DEX trades
CREATE TABLE IF NOT EXISTS swaps (
    id SERIAL PRIMARY KEY,
    tx_hash VARCHAR(66) NOT NULL,
    block_number BIGINT NOT NULL,
    log_index INT NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL,

    pair_address VARCHAR(42) NOT NULL,
    token_address VARCHAR(42) NOT NULL,
    wallet_address VARCHAR(42) NOT NULL,

    trade_type VARCHAR(4) NOT NULL CHECK (trade_type IN ('buy', 'sell')),
    amount_tokens DECIMAL(30, 18),
    amount_bnb DECIMAL(30, 18),
    amount_usd DECIMAL(30, 2),
    price_usd DECIMAL(30, 18),

    is_whale BOOLEAN DEFAULT FALSE,

    UNIQUE(tx_hash, log_index)
);

CREATE INDEX IF NOT EXISTS idx_swaps_token_time ON swaps(token_address, timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_swaps_wallet_time ON swaps(wallet_address, timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_swaps_whale ON swaps(is_whale, timestamp DESC) WHERE is_whale = TRUE;
CREATE INDEX IF NOT EXISTS idx_swaps_pair ON swaps(pair_address, timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_swaps_block ON swaps(block_number DESC);
