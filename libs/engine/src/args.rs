use std::time::Duration;

use alloy::primitives::Address;

pub struct Args {
    pub address: Address,
    pub event: String,
    pub from_block: u64,
    pub poll_interval: Duration,
}
