use crate::args::Args;
use alloy::primitives::{BlockHash, BlockNumber};
use alloy::rpc::types::{Block, Log};
use chain::rpc::NodeClient;
use eyre::{Report, Result};
use futures_util::stream::{self};
use futures_util::{Stream, StreamExt};
use std::convert::TryInto;
use std::pin::Pin;
use std::sync::Arc;
use store::model::{Checkpoint, Transfer};
use store::store::Store;
use sync::{consumer::Consumer, producer::Producer};
use tokio::sync::{Mutex, broadcast, mpsc};

pub async fn chunked_backfill(
    args: &Args,
    node_client: &NodeClient,
    store: Arc<Store>,
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
                &args.address,
                &args.event,
                checkpoint_number.into(),
                to_block_number_chunk.into(),
            )
            .await?;

        for log in &logs {
            let transfer: Transfer = log.try_into()?;
            println!("Consumed: {transfer:?}");
            store.insert_transfer(&transfer).await?;
        }

        checkpoint_number = to_block_number_chunk + 1;
    }

    Ok(final_checkpoint)
}

pub async fn spawn_consumer(
    rx: mpsc::Receiver<Result<Transfer, Report>>,
    shutdown_tx: broadcast::Sender<()>,
    store: Arc<Store>,
) -> tokio::task::JoinHandle<()> {
    // A closure that returns a future.
    let consumer_callback = move |consumed_transfer: Result<Transfer>| {
        let store_for_consumer = Arc::clone(&store);
        async move {
            match &consumed_transfer {
                Err(e) => eprintln!("Consumer Failed: {e:?}"),
                Ok(transfer) => match store_for_consumer.insert_transfer(transfer).await {
                    Err(e) => eprintln!("Consumer Failed: {e:?} |> {transfer:?}"),
                    Ok(_) => println!("Consumed: {transfer:?}"),
                },
            }
        }
    };

    // Spawn consumer: consumes transfers from rx (producer)
    Consumer::spawn(rx, shutdown_tx.clone(), consumer_callback)
}

pub async fn watch_logs_stream(
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
    tx: mpsc::Sender<Result<Transfer, Report>>,
    shutdown_tx: broadcast::Sender<()>,
    shared_logs_stream: Arc<Mutex<Pin<Box<impl Stream<Item = Log> + Send + 'static>>>>,
) -> tokio::task::JoinHandle<()> {
    // A closure that returns a future
    let producer_callback = move || {
        let logs_stream_for_producer = Arc::clone(&shared_logs_stream);
        async move {
            let mut locked_stream = logs_stream_for_producer.lock().await;
            match &locked_stream.next().await {
                Some(log) => {
                    drop(locked_stream);
                    log.try_into()
                }
                None => {
                    drop(locked_stream);
                    Err(Report::msg("Stream ended"))
                }
            }
        }
    };

    // Spawn producer: produces transfers received from logs stream and sends them to tx (consumer)
    Producer::spawn(tx, shutdown_tx, producer_callback)
}
