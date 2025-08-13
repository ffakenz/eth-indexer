use crate::args::Args;
use crate::processor::Processor;
use alloy::primitives::{BlockHash, BlockNumber};
use alloy::rpc::types::{Block, Log};
use chain::rpc::NodeClient;
use eyre::eyre;
use eyre::{Report, Result};
use futures_util::stream::{self};
use futures_util::{Stream, StreamExt};
use std::pin::Pin;
use std::sync::Arc;
use store::checkpoint::model::Checkpoint;
use store::checkpoint::store::Store as CheckpointStore;
use sync::{consumer::Consumer, producer::Producer};
use tokio::sync::{Mutex, broadcast, mpsc};

pub async fn chunked_backfill<T>(
    args: &Args,
    node_client: &NodeClient,
    checkpoint_store: Arc<CheckpointStore>,
    processor: Arc<dyn Processor<T>>,
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
        let to_block_number_chunk =
            std::cmp::min(checkpoint_number + args.backfill_chunk_size - 1, latest_block_number);

        let logs: Vec<Log> = node_client
            .get_logs(
                args.addresses.clone(),
                &args.event,
                checkpoint_number.into(),
                to_block_number_chunk.into(),
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

        match save_checkpoint(to_block_number_chunk, node_client, &checkpoint_store).await {
            Ok(success) => success,
            Err(e) => {
                eprintln!("Backfill failed on [save_checkpoint]: {e:?}");
                return Err(eyre!(e));
            }
        }

        checkpoint_number = to_block_number_chunk + 1;
    }

    Ok(final_checkpoint)
}

pub async fn save_checkpoint(
    block_number: BlockNumber,
    node_client: &NodeClient,
    checkpoint_store: &CheckpointStore,
) -> Result<()> {
    // Fetch latest processed block to build checkpoint
    let block = node_client
        .get_block_by_number(block_number)
        .await?
        .ok_or_else(|| eyre!("Block not found for checkpoint: {}", block_number))?;

    let checkpoint = Checkpoint {
        block_number: block_number as i64,
        block_hash: block.hash().to_vec(),
        parent_hash: block.header.parent_hash.to_vec(),
    };

    checkpoint_store.insert_checkpoint(checkpoint).await?;
    println!("Checkpoint saved at block number {block_number:?} and hash {}", block.hash());
    Ok(())
}

pub async fn spawn_consumer<T: Send + Sync + 'static>(
    rx: mpsc::Receiver<Result<Log, Report>>,
    shutdown_tx: broadcast::Sender<()>,
    node_client: &NodeClient,
    checkpoint_store: Arc<CheckpointStore>,
    processor: Arc<dyn Processor<T>>,
) -> tokio::task::JoinHandle<()> {
    // A closure that returns a future.
    let node_client_cloned = node_client.clone();
    let shutdown_tx_cloned = shutdown_tx.clone();
    let consumer_callback = move |consumed_log: Result<Log>| {
        let checkpoint_store_for_consumer: Arc<CheckpointStore> = Arc::clone(&checkpoint_store);
        let processor_for_consumer: Arc<dyn Processor<T>> = Arc::clone(&processor);
        let node_client_for_consumer = node_client_cloned.clone();
        let shutdown_tx_for_consumerr = shutdown_tx_cloned.clone();
        async move {
            match &consumed_log {
                Err(e) => {
                    eprintln!("Consumer received failed signal from Producer: {e:?}");
                    // stop signal
                    let _ = shutdown_tx_for_consumerr.send(());
                }
                Ok(log) => {
                    println!("Consumer consumed: {log:?}");
                    match log.block_number {
                        // NOTE: This log comes from a pending tx that has not yet been mined.
                        // Pending logs are re-emitted (with the same tx hash and log index)
                        // once the tx is included in a block, at which point `block_number` will be set.
                        // We skip it (ignore) now and process it only after confirmation.
                        None => (),
                        // the tx that emitted this log has been mined into a block
                        Some(block_number) => {
                            match processor_for_consumer.process_log(log).await {
                                Ok(_) => (),
                                Err(_) => {
                                    // stop signal
                                    let _ = shutdown_tx_for_consumerr.send(());
                                }
                            };
                            match save_checkpoint(
                                block_number,
                                &node_client_for_consumer,
                                &checkpoint_store_for_consumer,
                            )
                            .await
                            {
                                Ok(success) => success,
                                Err(e) => {
                                    eprintln!("Consumer failed on [save_checkpoint]: {e:?}");
                                    // stop signal
                                    let _ = shutdown_tx_for_consumerr.send(());
                                }
                            }
                        }
                    }
                }
            }
        }
    };

    // Spawn consumer: consumes logs from rx (producer)
    Consumer::spawn(rx, shutdown_tx.clone(), consumer_callback)
}

pub async fn watch_logs_stream(
    args: &Args,
    node_client: &NodeClient,
    checkpoint_number: BlockNumber,
) -> Result<Arc<Mutex<Pin<Box<impl Stream<Item = Log> + Send + 'static>>>>> {
    let logs_stream = node_client
        .watch_logs(
            args.addresses.clone(),
            &args.event,
            checkpoint_number.into(),
            args.poll_interval,
        )
        .await?
        .flat_map(stream::iter);

    // Box::pin(stream), pins the stream on the heap to guarantee it doesn't move, so it can be safely polled.
    // Wrap the stream in a Arc + Mutex for interior mutability.
    // * Arc, allows sharing across async tasks/closures.
    // * Mutex, gives async mutable access:
    let shared_logs_stream = Arc::new(Mutex::new(Box::pin(logs_stream)));
    Ok(shared_logs_stream)
}

pub async fn spawn_producer(
    tx: mpsc::Sender<Result<Log, Report>>,
    shutdown_tx: broadcast::Sender<()>,
    shared_logs_stream: Arc<Mutex<Pin<Box<impl Stream<Item = Log> + Send + 'static>>>>,
) -> tokio::task::JoinHandle<()> {
    // A closure that returns a future
    let producer_callback = move || {
        let logs_stream_for_producer = Arc::clone(&shared_logs_stream);
        async move {
            let mut locked_stream = logs_stream_for_producer.lock().await;
            match locked_stream.next().await {
                Some(log) => {
                    drop(locked_stream);
                    Ok(log)
                }
                None => {
                    drop(locked_stream);
                    Err(Report::msg("Stream ended"))
                }
            }
        }
    };

    // Spawn producer: produces received from logs stream and sends them to tx (consumer)
    Producer::spawn(tx, shutdown_tx, producer_callback)
}
