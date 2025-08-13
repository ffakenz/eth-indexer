use crate::args::Args;
use crate::processor::handle::Processor;
use alloy::rpc::types::Log;
use chain::rpc::NodeClient;
use eyre::Result;
use std::sync::Arc;
use store::checkpoint::store::Store as CheckpointStore;
use tokio::sync::{broadcast, mpsc};
use tokio::task::JoinHandle;

pub struct Engine {
    shutdown_tx: broadcast::Sender<()>,
    consumer_handle: JoinHandle<()>,
    producer_handle: JoinHandle<()>,
}

impl Engine {
    pub async fn start<T: Send + Sync + 'static>(
        args: &Args,
        node_client: &NodeClient,
        checkpoint_store: Arc<CheckpointStore>,
        processor: Arc<dyn Processor<Log, T>>,
    ) -> Result<Engine> {
        // 1. Run backfill synchronously, collect logs gap-fill

        let checkpoint = crate::utils::chunked_backfill(
            args,
            node_client,
            Arc::clone(&checkpoint_store),
            Arc::clone(&processor),
        )
        .await?;

        // 2. Run watch asynchronously, collect logs live

        // Channel for passing logs from producer to consumer
        let (tx, rx) = mpsc::channel::<Result<Log>>(100);

        // Broadcast channel for shutdown signal
        let (shutdown_tx, _) = broadcast::channel::<()>(1);

        // -- Spawn Consumer --
        let consumer_handle = crate::utils::spawn_consumer(
            rx,
            shutdown_tx.clone(),
            node_client,
            Arc::clone(&checkpoint_store),
            Arc::clone(&processor),
        )
        .await;

        // -- Spawn Producer --
        let next_checkpoint_number = (checkpoint.block_number + 1) as u64;
        let shared_logs_stream =
            crate::utils::watch_logs_stream(args, node_client, next_checkpoint_number).await?;
        let producer_handle =
            crate::utils::spawn_producer(tx, shutdown_tx.clone(), Arc::clone(&shared_logs_stream))
                .await;

        Ok(Self { shutdown_tx, consumer_handle, producer_handle })
    }

    // Send shutdown signal and wait for both producer and consumer to finish
    pub async fn shutdown(self) {
        // Send shutdown signal
        let _ = self.shutdown_tx.send(());

        // Await tasks
        let _ = self.producer_handle.await;
        let _ = self.consumer_handle.await;
    }
}
