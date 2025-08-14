use std::time::Duration;

use alloy::{
    primitives::{Address, BlockHash},
    rpc::types::ValueOrArray,
};

// REVIEW! should backfill_chunk_size and checkpoint_interval be the same?
pub struct Args {
    pub addresses: ValueOrArray<Address>,
    pub event: String,
    pub from_block: BlockHash,
    // positive number
    pub backfill_chunk_size: u64,
    // positive number: N logs between checkpoints
    pub checkpoint_interval: u64,
    pub poll_interval: Duration,
}
