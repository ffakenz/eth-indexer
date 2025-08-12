use crate::args::Args;
use alloy::rpc::types::Log;
use chain::rpc::NodeClient;
use eyre::Result;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::{Mutex, broadcast, mpsc};
use tokio::task::JoinHandle;
pub struct Engine {
    shutdown_tx: broadcast::Sender<()>,
    consumer_handle: JoinHandle<()>,
    producer_handle: JoinHandle<()>,
    collected_logs: Arc<Mutex<VecDeque<Log>>>,
}

impl Engine {
    pub async fn start(args: Args, node_client: &NodeClient) -> Result<Engine> {
        // 1. Run backfill synchronously, collect logs gap-fill

        let (collected_logs, checkpoint_number, checkpoint_hash) =
            crate::utils::chunked_backfill(&args, node_client, args.backfill_chunk_size).await?;

        // 2. Run watch asynchronously, collect logs live

        // Channel for passing logs from producer to consumer
        let (tx, rx) = mpsc::channel::<Result<Log>>(100);

        // Broadcast channel for shutdown signal
        let (shutdown_tx, _) = broadcast::channel::<()>(1);

        // FIXME! replace by persistent store (SQLite)
        // Wrapped in a Arc + Mutex for interior mutability.
        // * Arc, allows sharing across async tasks/closures.
        // * Mutex, gives async mutable access
        let shared_collected_logs = Arc::new(Mutex::new(collected_logs));
        let shared_checkpoint_hash = Arc::new(Mutex::new(checkpoint_hash));

        // -- Spawn Consumer --
        let consumer_handle = crate::utils::spawn_consumer(
            rx,
            shutdown_tx.clone(),
            Arc::clone(&shared_collected_logs),
            Arc::clone(&shared_checkpoint_hash),
        )
        .await;

        // -- Spawn Producer --
        let next_checkpoint_number = checkpoint_number + 1;
        let shared_logs_stream =
            crate::utils::watch_logs_stream(&args, node_client, next_checkpoint_number).await?;
        let producer_handle =
            crate::utils::spawn_producer(tx, shutdown_tx.clone(), Arc::clone(&shared_logs_stream))
                .await;

        Ok(Self {
            shutdown_tx,
            consumer_handle,
            producer_handle,
            collected_logs: Arc::clone(&shared_collected_logs),
        })
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
    pub async fn get_collected_logs(&self) -> VecDeque<Log> {
        let locked = self.collected_logs.lock().await;
        let collected_logs_cloned = locked.clone();
        drop(locked);
        collected_logs_cloned
    }
}
