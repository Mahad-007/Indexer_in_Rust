-- Alert events queue for notifications
CREATE TABLE IF NOT EXISTS alert_events (
    id SERIAL PRIMARY KEY,
    created_at TIMESTAMPTZ DEFAULT NOW(),

    alert_type VARCHAR(50) NOT NULL,
    token_address VARCHAR(42),
    token_symbol VARCHAR(50),
    wallet_address VARCHAR(42),

    title VARCHAR(255) NOT NULL,
    message TEXT,

    -- Alert metadata
    bee_score SMALLINT,
    amount_usd DECIMAL(30, 2),
    change_percent DECIMAL(10, 4),

    -- Extra data as JSON for flexibility
    metadata JSONB,

    -- Processing status
    processed BOOLEAN DEFAULT FALSE,
    processed_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_alerts_unprocessed ON alert_events(processed, created_at) WHERE processed = FALSE;
CREATE INDEX IF NOT EXISTS idx_alerts_type ON alert_events(alert_type, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_alerts_token ON alert_events(token_address, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_alerts_created ON alert_events(created_at DESC);
