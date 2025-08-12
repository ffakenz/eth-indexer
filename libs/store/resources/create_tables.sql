-- Table: transfers

CREATE TABLE IF NOT EXISTS transfers (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    block_number        INTEGER NOT NULL,
    block_hash          BLOB NOT NULL,
    transaction_hash    BLOB NOT NULL,
    log_index           INTEGER NOT NULL,
    contract_address    BLOB NOT NULL,
    from_address        BLOB NOT NULL,
    to_address          BLOB NOT NULL,
    amount              BLOB NOT NULL,

    -- Constraint to ensure no duplicate logs
    UNIQUE(transaction_hash, log_index)
);

CREATE INDEX IF NOT EXISTS idx_transfers_block_number
    ON transfers (block_number);

CREATE INDEX IF NOT EXISTS idx_transfers_block_hash
    ON transfers (block_hash);

-------------------------------------------------------------

-- Table: checkpoints

CREATE TABLE IF NOT EXISTS checkpoints (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    block_number    INTEGER NOT NULL,
    block_hash      BLOB NOT NULL,
    parent_hash     BLOB NOT NULL,

    -- Each block should appear only once
    UNIQUE(block_number, block_hash)
);

CREATE INDEX IF NOT EXISTS idx_checkpoints_block_number
    ON checkpoints (block_number DESC);

CREATE INDEX IF NOT EXISTS idx_checkpoints_block_hash
    ON checkpoints (block_hash DESC);
