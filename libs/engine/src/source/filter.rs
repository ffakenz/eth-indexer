use std::time::Duration;

use alloy::{eips::BlockNumberOrTag, primitives::Address, rpc::types::ValueOrArray};

#[derive(Clone)]
pub enum EventType {
    Transfer,
}

impl EventType {
    // Here we have declared a string slice initialized with a string literal.
    // String literals have a static lifetime, which means the string is guaranteed
    // to be valid for the duration of the entire program.
    pub fn as_str(&self) -> &'static str {
        match self {
            EventType::Transfer => "Transfer(address,address,uint256)",
        }
    }
}

pub struct ChunkFilter {
    pub addresses: ValueOrArray<Address>,
    pub event: EventType,
    pub from_block_number: BlockNumberOrTag,
    pub to_block_number: BlockNumberOrTag,
}

pub struct StreamFilter {
    pub addresses: ValueOrArray<Address>,
    pub event: EventType,
    pub from_block_number: BlockNumberOrTag,
    pub poll_interval: Duration,
}
