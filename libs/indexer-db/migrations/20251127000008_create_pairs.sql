-- Pairs table for tracking DEX trading pairs
CREATE TABLE IF NOT EXISTS pairs (
    id SERIAL PRIMARY KEY,
    address VARCHAR(42) NOT NULL UNIQUE,
    token0_address VARCHAR(42) NOT NULL,
    token1_address VARCHAR(42) NOT NULL,
    factory_address VARCHAR(42) NOT NULL,

    -- Reserve tracking for price calculations
    reserve0 DECIMAL(78, 0) DEFAULT 0,
    reserve1 DECIMAL(78, 0) DEFAULT 0,

    -- Which token is WBNB/base (0 or 1)
    base_token_index SMALLINT DEFAULT 0,

    block_number BIGINT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    last_updated TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_pairs_address ON pairs(address);
CREATE INDEX IF NOT EXISTS idx_pairs_token0 ON pairs(token0_address);
CREATE INDEX IF NOT EXISTS idx_pairs_token1 ON pairs(token1_address);
CREATE INDEX IF NOT EXISTS idx_pairs_created ON pairs(created_at DESC);
