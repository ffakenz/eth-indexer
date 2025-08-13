use crate::args::Args;
use crate::checkpointer;
use crate::processor::handle::Processor;
use alloy::primitives::{BlockHash, BlockNumber};
use alloy::rpc::types::{Block, Log};
use chain::rpc::NodeClient;
use eyre::eyre;
use eyre::{Report, Result};
use std::sync::Arc;
use store::checkpoint::model::Checkpoint;
use store::checkpoint::store::Store as CheckpointStore;

pub async fn chunked_backfill<T>(
    args: &Args,
    node_client: &NodeClient,
    checkpoint_store: Arc<CheckpointStore>,
    processor: Arc<dyn Processor<Log, T>>,
) -> Result<Checkpoint> {
    // Lookup checkpoint block
    let checkpoint_block: Block = node_client
        .get_block_by_hash(args.from_block)
        .await?
        .ok_or_else(|| Report::msg(format!("Checkpoint block not found: {:?}", args.from_block)))?;

    // Local mut state
    let mut checkpoint_number: BlockNumber = checkpoint_block.number();

    // Lookup latest block
    let latest_block: Block = node_client
        .get_latest_block()
        .await?
        .ok_or_else(|| Report::msg("Latest block not found"))?;

    let latest_block_number: BlockNumber = latest_block.number();
    let latest_block_hash: BlockHash = latest_block.hash();

    let final_checkpoint = Checkpoint {
        block_number: latest_block_number as i64,
        block_hash: latest_block_hash.to_vec(),
        parent_hash: latest_block.header.parent_hash.to_vec(),
    };

    // Process historical chunks until we reach the snapshot tip
    while checkpoint_number <= latest_block_number {
        let chunk_block_number =
            std::cmp::min(checkpoint_number + args.backfill_chunk_size - 1, latest_block_number);

        // Fetch latest processed block to have it ready to build checkpoint a checkpoint
        // as soon as we complete processing logs.
        let chunk_checkpoint_block = node_client
            .get_block_by_number(chunk_block_number)
            .await?
            .ok_or_else(|| eyre!("Block not found for checkpoint: {}", chunk_block_number))?;

        let logs: Vec<Log> = node_client
            .get_logs(
                args.addresses.clone(),
                &args.event,
                checkpoint_number.into(),
                chunk_block_number.into(),
            )
            .await?
            .into_iter()
            // NOTE: Logs may come from pending txs that have not yet been mined.
            // Pending logs are re-emitted (with the same tx hash and log index)
            // once their tx is included in a block, at which point `block_number` will be set.
            // We skip them (ignore) here to process only confirmed logs in backfill mode.
            .filter(|log| log.block_number.is_some())
            .collect();

        for log in &logs {
            match processor.process_log(log).await {
                Ok(_) => continue,
                Err(err) => return Err(err),
            }
        }

        match checkpointer::save_checkpoint(&chunk_checkpoint_block, &checkpoint_store).await {
            Ok(success) => success,
            Err(e) => {
                eprintln!("Backfill failed on [save_checkpoint]: {e:?}");
                return Err(eyre!(e));
            }
        }

        checkpoint_number = chunk_block_number + 1;
    }

    Ok(final_checkpoint)
}
