-- Tokens table (core entity for BeanBee)
CREATE TABLE IF NOT EXISTS tokens (
    id SERIAL PRIMARY KEY,
    address VARCHAR(42) NOT NULL UNIQUE,
    name VARCHAR(255),
    symbol VARCHAR(50),
    decimals SMALLINT DEFAULT 18,
    total_supply DECIMAL(78, 0),
    pair_address VARCHAR(42),
    creator_address VARCHAR(42),
    created_at TIMESTAMPTZ,
    block_number BIGINT,

    -- Live metrics
    price_usd DECIMAL(30, 18) DEFAULT 0,
    price_bnb DECIMAL(30, 18) DEFAULT 0,
    price_change_1h DECIMAL(10, 4) DEFAULT 0,
    price_change_24h DECIMAL(10, 4) DEFAULT 0,
    market_cap_usd DECIMAL(30, 2) DEFAULT 0,
    liquidity_usd DECIMAL(30, 2) DEFAULT 0,
    liquidity_bnb DECIMAL(30, 18) DEFAULT 0,
    volume_1h_usd DECIMAL(30, 2) DEFAULT 0,
    volume_24h_usd DECIMAL(30, 2) DEFAULT 0,
    trades_1h INT DEFAULT 0,
    trades_24h INT DEFAULT 0,
    buys_1h INT DEFAULT 0,
    sells_1h INT DEFAULT 0,

    -- Holder metrics
    holder_count INT DEFAULT 0,
    holder_count_1h_ago INT DEFAULT 0,
    top_10_holder_percent DECIMAL(5, 2) DEFAULT 0,
    dev_holdings_percent DECIMAL(5, 2) DEFAULT 0,
    sniper_ratio DECIMAL(5, 2) DEFAULT 0,

    -- Safety flags
    lp_locked BOOLEAN DEFAULT FALSE,
    lp_lock_percent DECIMAL(5, 2) DEFAULT 0,
    lp_unlock_date TIMESTAMPTZ,
    ownership_renounced BOOLEAN DEFAULT FALSE,

    -- BeeScore
    bee_score SMALLINT DEFAULT 0 CHECK (bee_score >= 0 AND bee_score <= 100),
    safety_score SMALLINT DEFAULT 0 CHECK (safety_score >= 0 AND safety_score <= 60),
    traction_score SMALLINT DEFAULT 0 CHECK (traction_score >= 0 AND traction_score <= 40),

    last_updated TIMESTAMPTZ DEFAULT NOW(),
    indexed_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_tokens_bee_score ON tokens(bee_score DESC);
CREATE INDEX IF NOT EXISTS idx_tokens_created_at ON tokens(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_tokens_volume ON tokens(volume_1h_usd DESC);
CREATE INDEX IF NOT EXISTS idx_tokens_address ON tokens(address);
CREATE INDEX IF NOT EXISTS idx_tokens_pair_address ON tokens(pair_address);
CREATE INDEX IF NOT EXISTS idx_tokens_symbol ON tokens(symbol);
