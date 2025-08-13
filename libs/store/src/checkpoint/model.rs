use sqlx::FromRow;

#[derive(Clone, FromRow, PartialEq)]
pub struct Checkpoint {
    pub block_number: i64,
    pub block_hash: Vec<u8>,
    pub parent_hash: Vec<u8>,
}
