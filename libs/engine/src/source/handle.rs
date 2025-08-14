use std::time::Duration;

use alloy::{
    eips::BlockNumberOrTag,
    primitives::{Address, BlockNumber},
    rpc::types::ValueOrArray,
};
use eyre::Result;
use futures_util::stream::BoxStream;

pub struct ChunkFilter {
    pub addresses: ValueOrArray<Address>,
    pub event: String,
    pub from_block_number: BlockNumberOrTag,
    pub to_block_number: BlockNumberOrTag,
}

pub struct StreamFilter {
    pub addresses: ValueOrArray<Address>,
    pub event: String,
    pub from_block_number: BlockNumberOrTag,
    pub poll_interval: Duration,
}

pub trait SourceInput {
    fn block_number(&self) -> Option<BlockNumber>;
}

#[async_trait::async_trait]
pub trait Source {
    type Item: SourceInput;

    async fn chunk(&self, filter: ChunkFilter) -> Result<Vec<Self::Item>>;
    async fn stream(&self, filter: StreamFilter) -> Result<BoxStream<'static, Self::Item>>;
}
