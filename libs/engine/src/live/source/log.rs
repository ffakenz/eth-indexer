use crate::live::source::filter::{ChunkFilter, StreamFilter};
use crate::live::source::handle::{Source, SourceInput};
use alloy::{primitives::BlockNumber, rpc::types::Log};
use chain::rpc::NodeClient;
use eyre::Result;
use futures_util::{
    StreamExt, future,
    stream::{self, BoxStream},
};

pub struct LogSource {
    pub node_client: NodeClient,
}

impl SourceInput for Log {
    fn block_number(&self) -> Option<BlockNumber> {
        self.block_number
    }
}

#[async_trait::async_trait]
impl Source for LogSource {
    type Item = Log;

    async fn chunk(&self, filter: ChunkFilter) -> Result<Vec<Self::Item>> {
        let logs_chunk = self
            .node_client
            .get_logs(
                filter.addresses,
                filter.event.as_str(),
                filter.from_block_number,
                filter.to_block_number,
            )
            .await?
            .into_iter()
            // NOTE: Logs may come from pending txs that have not yet been mined.
            // Pending logs are re-emitted (with the same tx hash and log index)
            // once their tx is included in a block, at which point `block_number` will be set.
            // We skip them (ignore) here to process only confirmed logs in backfill mode.
            .filter(|log| log.block_number.is_some())
            .collect();

        Ok(logs_chunk)
    }

    async fn stream(&self, filter: StreamFilter) -> Result<BoxStream<'static, Self::Item>> {
        let logs_stream = self
            .node_client
            .watch_logs(
                filter.addresses,
                filter.event.as_str(),
                filter.from_block_number,
                filter.poll_interval,
            )
            .await?
            .flat_map(stream::iter)
            // NOTE: This log comes from a pending tx that has not yet been mined.
            // Pending logs are re-emitted (with the same tx hash and log index)
            // once the tx is included in a block, at which point `block_number` will be set.
            // We skip it (ignore) now and process it only after confirmation.
            .filter_map(|log| {
                future::ready(if log.block_number().is_some() { Some(log) } else { None })
            });

        Ok(Box::pin(logs_stream))
    }
}
