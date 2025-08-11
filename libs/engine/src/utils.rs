use crate::args::Args;
use alloy::primitives::{BlockHash, BlockNumber};
use alloy::rpc::types::{Block, Log};
use chain::rpc::NodeClient;
use eyre::{Report, Result};
use futures_util::stream::{self};
use futures_util::{Stream, StreamExt};
use std::collections::VecDeque;
use std::pin::Pin;
use std::sync::Arc;
use sync::{consumer::Consumer, producer::Producer};
use tokio::sync::{Mutex, broadcast, mpsc};

pub async fn gap_fill(
    args: &Args,
    node_client: &NodeClient,
) -> Result<(VecDeque<Log>, BlockNumber, BlockHash)> {
    // Lookup checkpoint block
    let checkpoint_block: Block = node_client
        .get_block_by_hash(args.from_block)
        .await?
        .ok_or_else(|| Report::msg(format!("Checkpoint block not found: {:?}", args.from_block)))?;

    // Local mut state
    let mut checkpoint_hash: BlockHash = checkpoint_block.hash();
    let mut checkpoint_number: BlockNumber = checkpoint_block.number();
    let mut collected_logs: VecDeque<Log> = VecDeque::new();

    // Moonwalk blocks and logs from checkpoint
    let blocks: Vec<Block> = node_client.moonwalk_blocks(checkpoint_hash).await?;
    for block in &blocks {
        let block_logs =
            node_client.filtered_tx_logs_from_block(block, &args.address, &args.event).await?;
        for log in block_logs {
            // push_front because moonwalk is FIFO
            collected_logs.push_front(log);
        }
        // Update checkpoint to latest block hash after processing last log
        checkpoint_hash = block.hash();
        checkpoint_number = block.number();
    }

    // Consume moonwalked logs
    for log in &collected_logs {
        // TODO! persist log + checkpoint into store (SQLite)
        println!("Consumed log: {log:?}");
    }

    Ok((collected_logs, checkpoint_number, checkpoint_hash))
}

pub async fn spawn_consumer(
    rx: mpsc::Receiver<Result<Log, Report>>,
    shutdown_tx: broadcast::Sender<()>,
    shared_collected_logs: Arc<Mutex<VecDeque<Log>>>,
    shared_checkpoint_hash: Arc<Mutex<BlockHash>>,
) -> tokio::task::JoinHandle<()> {
    // A closure that returns a future.
    let consumer_callback = move |consumed_log: Result<Log>| {
        let collected_logs_for_consumer = Arc::clone(&shared_collected_logs);
        let checkpoint_hash_for_consumer = Arc::clone(&shared_checkpoint_hash);
        async move {
            let mut locked_collected_logs = collected_logs_for_consumer.lock().await;
            let mut locked_checkpoint_hash = checkpoint_hash_for_consumer.lock().await;
            if let Ok(log) = consumed_log {
                if let Some(log_hash) = log.block_hash {
                    // TODO! persist log + checkpoint into store (SQLite)
                    println!("Consumed log: {log:?}");
                    *locked_checkpoint_hash = log_hash;
                    locked_collected_logs.push_back(log);
                    drop(locked_collected_logs);
                    drop(locked_checkpoint_hash);
                }
            }
        }
    };

    // Spawn consumer: consumes logs from rx (producer)
    Consumer::spawn(rx, shutdown_tx.clone(), consumer_callback)
}

pub async fn get_logs_stream(
    args: &Args,
    node_client: &NodeClient,
    checkpoint_number: BlockNumber,
) -> Result<Arc<Mutex<Pin<Box<impl Stream<Item = Log> + Send + 'static>>>>> {
    let logs_stream = node_client
        .watch_logs(&args.address, &args.event, checkpoint_number.into(), args.poll_interval)
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

    // Spawn producer: produces logs received from stream and sends them to tx (consumer)
    Producer::spawn(tx, shutdown_tx, producer_callback)
}
