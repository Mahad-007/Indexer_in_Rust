-- Wallets table for tracking wallets with labels and metadata
CREATE TABLE IF NOT EXISTS wallets (
    id SERIAL PRIMARY KEY,
    address VARCHAR(42) NOT NULL UNIQUE,
    label VARCHAR(255),
    
    -- Computed stats (can be updated periodically)
    token_count INT DEFAULT 0,
    estimated_value_usd DECIMAL(30, 2) DEFAULT 0,
    last_activity TIMESTAMPTZ,
    
    -- Metadata
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_wallets_address ON wallets(address);
CREATE INDEX IF NOT EXISTS idx_wallets_last_activity ON wallets(last_activity DESC);
CREATE INDEX IF NOT EXISTS idx_wallets_estimated_value ON wallets(estimated_value_usd DESC);

