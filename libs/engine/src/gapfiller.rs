use crate::args::Args;
use crate::checkpointer::Checkpointer;
use crate::live::sink::handle::Sink;
use crate::live::source::filter::ChunkFilter;
use crate::live::source::handle::{Source, SourceInput};
use alloy::primitives::BlockNumber;
use alloy::rpc::types::Block;
use chain::rpc::NodeClient;
use eyre::{Result, eyre};
use std::fmt::Debug;
use store::checkpoint::model::Checkpoint;

pub async fn chunked_backfill<E, T>(
    args: &Args,
    node_client: &NodeClient,
    source: &dyn Source<Item = E>,
    checkpointer: &Checkpointer,
    sink: &dyn Sink<Item = T>,
) -> Result<Checkpoint>
where
    E: SourceInput + Debug + Clone,
    <E as TryInto<T>>::Error: Debug,
    T: TryFrom<E>,
{
    // Lookup latest block
    let latest_block: Block =
        node_client.get_latest_block().await?.ok_or_else(|| eyre!("Latest block not found"))?;

    let latest_block_number: BlockNumber = latest_block.number();

    let checkpoint_interval = match args.backfill_checkpoint_interval {
        None => args.checkpoint_interval,
        Some(backfill_checkpoint_interval) => backfill_checkpoint_interval,
    };

    // Local mut state
    let mut checkpoint_number: BlockNumber = match args.from_block {
        Some(from_block_number) => from_block_number,
        None => match checkpointer.get_last_checkpoint().await? {
            Some(checkpoint_block) => checkpoint_block.block_number as u64,
            // start from the tip
            None => latest_block_number,
        },
    };

    tracing::info!("Backfill started at block number: {checkpoint_number:?}");

    // Process historical chunks until we reach the snapshot tip
    while checkpoint_number <= latest_block_number {
        // A safe checked addition avoids silent wraparound
        let chunk_block_number =
            checkpoint_number.saturating_add(checkpoint_interval - 1).min(latest_block_number);

        // Fetch latest processed block to have it ready to build checkpoint a checkpoint
        // as soon as we complete processing logs.
        let chunk_checkpoint_block = node_client
            .get_block_by_number(chunk_block_number)
            .await?
            .ok_or_else(|| eyre!("Block not found for checkpoint: {}", chunk_block_number))?;

        let chunk_filter = ChunkFilter {
            addresses: args.addresses.clone(),
            event: args.event.clone(),
            from_block_number: checkpoint_number.into(),
            to_block_number: chunk_block_number.into(),
        };
        let source_inputs = source.chunk(chunk_filter).await?;

        let elements: Vec<_> = source_inputs
            .into_iter()
            .filter_map(|input| match input.clone().try_into() {
                Ok(element) => Some(element),
                Err(e) => {
                    tracing::error!(
                        "Skip: Failed to convert sourced input: {input:?} - reason {e:?}"
                    );
                    None
                }
            })
            .collect();

        if !elements.is_empty() {
            sink.process_batch(&elements).await?;
        }

        match checkpointer.checkpoint(&chunk_checkpoint_block).await {
            Ok(success) => success,
            Err(e) => {
                tracing::error!("Backfill failed on [save_checkpoint]: {e:?}");
                return Err(eyre!(e));
            }
        }

        checkpoint_number = chunk_block_number + 1;
    }

    tracing::info!("Backfill finished at block number: {latest_block_number:?}");

    Ok(Checkpoint {
        block_number: latest_block_number as i64,
        block_hash: latest_block.hash().to_vec(),
        parent_hash: latest_block.header.parent_hash.to_vec(),
    })
}
