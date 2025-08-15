use std::time::Duration;

use alloy::{
    primitives::{Address, BlockHash},
    rpc::types::ValueOrArray,
};

pub struct Args {
    // Addresses filter to watch
    pub addresses: ValueOrArray<Address>,
    // Event filter to watch
    pub event: String,
    // Latest known block that has been checkpointed
    pub from_block: BlockHash,
    // Positive number of events handled between checkpoints
    pub checkpoint_interval: u64,
    // Throttling (rate-limit) node requests:
    // minimum time to wait between consecutive calls
    pub poll_interval: Duration,
}
