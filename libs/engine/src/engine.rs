use crate::args::{MoonwalkArgs, WatchLogsArgs};
use alloy::primitives::BlockHash;
use alloy::rpc::types::Log;
use chain::rpc::NodeClient;
use eyre::{Report, Result};
use futures_util::{StreamExt, stream};
use std::sync::Arc;
use sync::{consumer::Consumer, producer::Producer};
use tokio::sync::{Mutex, broadcast, mpsc};
use tokio::task::JoinHandle;

pub struct Engine {
    shutdown_tx: broadcast::Sender<()>,
    consumer_handle: JoinHandle<()>,
    producer_handle: JoinHandle<()>,
    collected_logs: Arc<Mutex<Vec<Log>>>,
}

impl Engine {
    pub async fn start_watch_logs(args: WatchLogsArgs, node_client: &NodeClient) -> Result<Engine> {
        // Channel for passing logs from producer to consumer
        let (tx, rx) = mpsc::channel::<Result<Log>>(100);

        // Broadcast channel for shutdown signal
        let (shutdown_tx, _) = broadcast::channel::<()>(1);

        // Wrap the vec in a Arc + Mutex for interior mutability.
        // * Arc, allows sharing across async tasks/closures.
        // * Mutex, gives async mutable access:
        //   because the vec needs to be mutated exclusively and the callback can be called concurrently.
        let collected_logs = Arc::new(Mutex::new(Vec::new()));

        // Consumer callback: receives Result<Log> and prints
        // A closure that returns a future.
        let shared_collected_logs = Arc::clone(&collected_logs);
        let consumer_callback = move |consumed_log: Result<Log>| {
            let logs_for_consumer = Arc::clone(&shared_collected_logs);
            async move {
                let mut locked_collected_logs = logs_for_consumer.lock().await;
                if let Ok(log) = consumed_log {
                    println!("Consumed log: {log:?}");
                    locked_collected_logs.push(log);
                }
            }
        };

        // Spawn consumer: consumes logs from rx
        let consumer_handle = Consumer::spawn(rx, shutdown_tx.clone(), consumer_callback);

        let logs_stream = node_client
            .watch_logs(&args.address, &args.event, args.from_block, args.poll_interval)
            .await?
            .flat_map(stream::iter);

        // Box::pin(stream), pins the stream on the heap to guarantee it doesn't move, so it can be safely polled.
        // Wrap the stream in a Arc + Mutex for interior mutability.
        // * Arc, allows sharing across async tasks/closures.
        // * Mutex, gives async mutable access:
        //   because the stream needs to be polled exclusively and the callback can be called concurrently.
        let shared_logs_stream = Arc::new(Mutex::new(Box::pin(logs_stream)));

        // Producer callback: receives Option<Log> and passes it to consumer
        // A closure thaOkreturns a future
        let producer_callback = move || {
            let logs_stream_for_producer = Arc::clone(&shared_logs_stream);
            async move {
                let mut locked_stream = logs_stream_for_producer.lock().await;
                match locked_stream.next().await {
                    Some(log) => Ok(log),
                    None => Err(Report::msg("Stream ended")),
                }
            }
        };

        // Spawn producer: produces logs and sends to tx
        let producer_handle = Producer::spawn(tx, shutdown_tx.clone(), producer_callback);

        Ok(Self { shutdown_tx, consumer_handle, producer_handle, collected_logs })
    }

    pub async fn start_moonwalk_logs(
        args: MoonwalkArgs,
        node_client: &NodeClient,
    ) -> Result<Engine> {
        // Channel for passing logs from producer to consumer
        let (tx, rx) = mpsc::channel::<Result<Log>>(100);

        // Broadcast channel for shutdown signal
        let (shutdown_tx, _) = broadcast::channel::<()>(1);

        // Wrap the vec in a Arc + Mutex for interior mutability.
        // * Arc, allows sharing across async tasks/closures.
        // * Mutex, gives async mutable access:
        //   because the vec needs to be mutated exclusively and the callback can be called concurrently.
        let collected_logs = Arc::new(Mutex::new(Vec::new()));

        // Consumer callback: receives Result<Log> and prints
        // A closure that returns a future.
        let shared_collected_logs = Arc::clone(&collected_logs);
        let consumer_callback = move |consumed_log: Result<Log>| {
            let logs_for_consumer = Arc::clone(&shared_collected_logs);
            async move {
                let mut locked_collected_logs = logs_for_consumer.lock().await;
                if let Ok(log) = consumed_log {
                    println!("Consumed log: {log:?}");
                    locked_collected_logs.push(log);
                }
            }
        };

        // Spawn consumer: consumes logs from rx
        let consumer_handle = Consumer::spawn(rx, shutdown_tx.clone(), consumer_callback);

        // Wrap the checkpoint in a Arc + Mutex for interior mutability.
        // * Arc, allows sharing across async tasks/closures.
        // * Mutex, gives async mutable access:
        //   because the checkpoint needs to be mutated exclusively and the callback can be called concurrently.
        let checkpoint = Arc::new(Mutex::new(args.from_block));

        let shared_node_client = node_client.clone();

        // Producer callback: receives Option<Log> and passes it to consumer
        // A closure thaOkreturns a future
        let producer_callback = move || {
            let checkpoint_for_producer = Arc::clone(&checkpoint);
            let node_client_for_producer = shared_node_client.clone();
            let args_for_producer = args.clone();
            async move {
                // Lock checkpoint mutex and read current checkpoint inside the closure
                let current_checkpoint_hash = {
                    let locked = checkpoint_for_producer.lock().await;
                    *locked
                };

                let blocks =
                    node_client_for_producer.moonwalk_blocks(current_checkpoint_hash).await?;

                let mut logs: Vec<(Log, BlockHash)> = vec![];
                for block in blocks {
                    let block_logs = node_client_for_producer
                        .filtered_tx_logs_from_block(
                            &block,
                            &args.address,
                            &args_for_producer.event,
                        )
                        .await?;
                    for log in block_logs {
                        logs.push((log, block.hash()));
                    }
                }

                if let Some((log, block_hash)) = logs.pop() {
                    // Update checkpoint to latest block hash after processing last log
                    let mut locked = checkpoint_for_producer.lock().await;
                    *locked = block_hash;
                    Ok(log)
                } else {
                    Err(Report::msg("Moonwalk found no blocks"))
                }
            }
        };

        // Spawn producer: produces logs and sends to tx
        let producer_handle = Producer::spawn(tx, shutdown_tx.clone(), producer_callback);

        Ok(Self { shutdown_tx, consumer_handle, producer_handle, collected_logs })
    }

    // Send shutdown signal and wait for both producer and consumer to finish
    pub async fn shutdown(self) {
        // Send shutdown signal
        let _ = self.shutdown_tx.send(());

        // Await tasks
        let _ = self.producer_handle.await;
        let _ = self.consumer_handle.await;
    }

    // Get a snapshot of the collected logs so far
    pub async fn get_collected_logs(&self) -> Vec<Log> {
        let locked = self.collected_logs.lock().await;
        locked.clone()
    }
}
