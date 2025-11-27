-- Price snapshots for historical charts
CREATE TABLE IF NOT EXISTS price_snapshots (
    id SERIAL PRIMARY KEY,
    token_address VARCHAR(42) NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL,

    price_usd DECIMAL(30, 18),
    price_bnb DECIMAL(30, 18),
    liquidity_usd DECIMAL(30, 2),
    volume_usd DECIMAL(30, 2),
    market_cap_usd DECIMAL(30, 2),
    holder_count INT,

    UNIQUE(token_address, timestamp)
);

CREATE INDEX IF NOT EXISTS idx_snapshots_token_time ON price_snapshots(token_address, timestamp DESC);

-- Partitioning hint: consider partitioning by timestamp for large datasets
-- CREATE TABLE price_snapshots_2024 PARTITION OF price_snapshots FOR VALUES FROM ('2024-01-01') TO ('2025-01-01');
