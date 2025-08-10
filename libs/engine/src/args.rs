use std::time::Duration;

use alloy::{eips::BlockNumberOrTag, primitives::Address};

pub struct Args {
    pub address: Address,
    pub event: String,
    pub from_block: BlockNumberOrTag,
    pub poll_interval: Duration,
}
