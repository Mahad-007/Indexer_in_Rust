-- Token holders tracking for top holder analysis
CREATE TABLE IF NOT EXISTS token_holders (
    id SERIAL PRIMARY KEY,
    token_address VARCHAR(42) NOT NULL,
    wallet_address VARCHAR(42) NOT NULL,

    balance DECIMAL(30, 18) DEFAULT 0,
    percent_of_supply DECIMAL(10, 6) DEFAULT 0,

    is_dev BOOLEAN DEFAULT FALSE,
    is_sniper BOOLEAN DEFAULT FALSE,
    is_contract BOOLEAN DEFAULT FALSE,

    first_buy_block BIGINT,
    last_updated TIMESTAMPTZ DEFAULT NOW(),

    UNIQUE(token_address, wallet_address)
);

CREATE INDEX IF NOT EXISTS idx_holders_token ON token_holders(token_address, balance DESC);
CREATE INDEX IF NOT EXISTS idx_holders_wallet ON token_holders(wallet_address);
CREATE INDEX IF NOT EXISTS idx_holders_dev ON token_holders(token_address, is_dev) WHERE is_dev = TRUE;
CREATE INDEX IF NOT EXISTS idx_holders_sniper ON token_holders(token_address, is_sniper) WHERE is_sniper = TRUE;
