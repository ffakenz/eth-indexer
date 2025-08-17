use std::time::Duration;

use alloy::{
    primitives::{Address, BlockNumber},
    rpc::types::ValueOrArray,
};

use crate::live::source::filter::EventType;

#[derive(Debug)]
pub struct Args {
    // Addresses filter to watch
    pub addresses: ValueOrArray<Address>,
    // Event filter to watch
    pub event: EventType,
    // Latest known block that has been checkpointed
    pub from_block: Option<BlockNumber>,
    // Positive number of events handled between checkpoints
    pub checkpoint_interval: u64,
    // Positive number of events handled between checkpoints
    pub backfill_checkpoint_interval: Option<u64>,
    // Throttling (rate-limit) node requests:
    // minimum time to wait between consecutive calls
    pub poll_interval: Duration,
}
