use sqlx::FromRow;

#[derive(Clone, FromRow, PartialEq, PartialOrd)]
pub struct Transfer {
    pub block_number: i64,
    pub block_hash: Vec<u8>,
    pub transaction_hash: Vec<u8>,
    pub log_index: i64,
    pub contract_address: Vec<u8>,
    pub from_address: Vec<u8>,
    pub to_address: Vec<u8>,
    pub amount: Vec<u8>,
}
