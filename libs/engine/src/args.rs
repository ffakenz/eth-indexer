use std::time::Duration;

use alloy::{
    primitives::{Address, BlockHash},
    rpc::types::ValueOrArray,
};

pub struct Args {
    pub addresses: ValueOrArray<Address>,
    pub event: String,
    pub from_block: BlockHash,
    pub backfill_chunk_size: u64,
    pub poll_interval: Duration,
}
