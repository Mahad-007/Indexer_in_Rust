-- LP Locks table for tracking liquidity locks
CREATE TABLE IF NOT EXISTS lp_locks (
    id SERIAL PRIMARY KEY,
    token_address VARCHAR(42) NOT NULL,
    pair_address VARCHAR(42) NOT NULL,
    lock_contract VARCHAR(42) NOT NULL,
    lock_contract_name VARCHAR(50), -- 'unicrypt', 'pinksale', 'mudra'

    locked_amount DECIMAL(30, 18),
    locked_percent DECIMAL(5, 2),
    lock_date TIMESTAMPTZ,
    unlock_date TIMESTAMPTZ,

    tx_hash VARCHAR(66),
    block_number BIGINT,
    is_active BOOLEAN DEFAULT TRUE,

    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_lp_locks_token ON lp_locks(token_address);
CREATE INDEX IF NOT EXISTS idx_lp_locks_pair ON lp_locks(pair_address);
CREATE INDEX IF NOT EXISTS idx_lp_locks_active ON lp_locks(is_active) WHERE is_active = TRUE;
CREATE INDEX IF NOT EXISTS idx_lp_locks_unlock ON lp_locks(unlock_date) WHERE is_active = TRUE;

-- Trigger for updating updated_at
CREATE TRIGGER update_lp_locks_updated_at
BEFORE UPDATE ON lp_locks
FOR EACH ROW
EXECUTE FUNCTION update_updated_at_column();
