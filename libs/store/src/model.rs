use sqlx::FromRow;

#[derive(Debug, Clone, FromRow)]
pub struct Transfer {
    pub block_number: i64,
    pub block_hash: Vec<u8>,
    pub transaction_hash: Vec<u8>,
    pub log_index: i64,
    // raw LogData as BLOB
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, FromRow)]
pub struct Checkpoint {
    pub block_number: i64,
    pub block_hash: Vec<u8>,
}
