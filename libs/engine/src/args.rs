use std::time::Duration;

use alloy::primitives::{Address, BlockHash};

#[derive(Clone)]
pub struct Args {
    pub address: Address,
    pub event: String,
    pub from_block: BlockHash,
    pub backfill_chunk_size: u64,
    pub poll_interval: Duration,
}
