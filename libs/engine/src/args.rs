use std::time::Duration;

use alloy::{
    eips::BlockNumberOrTag,
    primitives::{Address, BlockHash},
};

#[derive(Clone)]
pub struct WatchLogsArgs {
    pub address: Address,
    pub event: String,
    pub from_block: BlockNumberOrTag,
    pub poll_interval: Duration,
}

#[derive(Clone)]
pub struct MoonwalkArgs {
    pub address: Address,
    pub event: String,
    pub from_block: BlockHash,
}
