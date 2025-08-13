use alloy::rpc::types::{Block, Log};

#[derive(Debug)]
pub enum Event {
    Skip,
    Log(Box<Log>),
    Checkpoint(Box<Block>),
}
