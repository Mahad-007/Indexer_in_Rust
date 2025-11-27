-- Wallet activity for wallet tracker feature
CREATE TABLE IF NOT EXISTS wallet_activity (
    id SERIAL PRIMARY KEY,
    wallet_address VARCHAR(42) NOT NULL,
    tx_hash VARCHAR(66) NOT NULL,
    block_number BIGINT NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL,

    action VARCHAR(12) NOT NULL CHECK (action IN ('buy', 'sell', 'transfer_in', 'transfer_out')),
    token_address VARCHAR(42) NOT NULL,
    token_symbol VARCHAR(50),
    amount_tokens DECIMAL(30, 18),
    amount_usd DECIMAL(30, 2),

    UNIQUE(tx_hash, wallet_address, token_address, action)
);

CREATE INDEX IF NOT EXISTS idx_wallet_activity_wallet ON wallet_activity(wallet_address, timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_wallet_activity_token ON wallet_activity(token_address, timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_wallet_activity_block ON wallet_activity(block_number DESC);
