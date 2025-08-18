use alloy::primitives::BlockNumber;
use eyre::Result;
use futures_util::stream::BoxStream;

use crate::live::source::filter::{ChunkFilter, StreamFilter};

pub trait SourceInput {
    fn block_number(&self) -> Option<BlockNumber>;
}

#[async_trait::async_trait]
pub trait Source: Send + Sync {
    type Item: SourceInput;

    async fn chunk(&self, filter: ChunkFilter) -> Result<Vec<Self::Item>>;
    async fn stream(&self, filter: StreamFilter) -> Result<BoxStream<'static, Self::Item>>;
}
