use crate::args::Args;
use crate::checkpointer::Checkpointer;
use crate::gapfiller;
use crate::live::pubsub::{publisher, subscriber};
use crate::live::sink::handle::Sink;
use crate::live::source::handle::{Source, SourceInput};
use crate::live::state::event::Events;
use crate::live::state::logic;
use crate::live::state::outcome::Outcome;
use alloy::rpc::types::Block;
use chain::rpc::NodeClient;
use eyre::{Result, eyre};
use std::fmt::Debug;
use std::sync::Arc;
use tokio::sync::{Mutex, broadcast, mpsc};
use tokio::task::JoinHandle;

pub struct Engine {
    shutdown_tx: broadcast::Sender<()>,
    consumer_handle: JoinHandle<()>,
    producer_handle: JoinHandle<()>,
}

fn channel_size(args: &Args) -> usize {
    // safety multiplier to handle bursts
    let burst_factor: usize = 2;
    // expected block/event rate
    let avg_events_per_block: usize = 50;
    // number of blocks to wait before considering a block irreversible
    let finality: usize = 12;
    let channel_size: usize = std::cmp::max(finality, args.checkpoint_interval as usize)
        * avg_events_per_block
        * burst_factor;
    channel_size
}

impl Engine {
    pub async fn start<E, T>(
        args: &Args,
        node_client: &NodeClient,
        source: Arc<dyn Source<Item = E>>,
        checkpointer: &Checkpointer,
        sink: Arc<dyn Sink<Item = T>>,
    ) -> Result<Engine>
    where
        E: SourceInput + Debug + Clone + Send + Sync + 'static,
        <E as TryInto<T>>::Error: Debug + Send + Sync + 'static,
        T: Outcome + TryFrom<E> + Debug + Send + Sync + 'static,
    {
        // Lookup latest block
        let block_tip: Block =
            node_client.get_latest_block().await?.ok_or_else(|| eyre!("Latest block not found"))?;
        let tip_number = block_tip.number();
        tracing::info!("Engine started at block tip number: {tip_number:?}");

        let state = logic::init_state(&block_tip, args.from_block, checkpointer).await?;

        // Wrap in a Arc + Mutex for interior mutability.
        // * Arc, allows sharing across async tasks/closures.
        // * Mutex, gives async mutable access:
        let shared_state = Arc::new(Mutex::new(state));

        // 1. Run collect elements in chunks async (gap-fill)
        let from_block_number = shared_state.lock().await.get_current_block_number();
        tracing::info!("Backfill started at block number: {from_block_number:?}");

        let (gapfill_tx, gapfill_rx) = mpsc::channel::<Result<Events<T>>>(1);

        let (gapfill_shutdown_tx, _) = broadcast::channel::<()>(1);

        let gapfill_consumer_handle = subscriber::spawn_event_consumer(
            gapfill_rx,
            gapfill_shutdown_tx.clone(),
            Arc::new(checkpointer.clone()),
            Arc::clone(&sink),
        )
        .await;

        let gapfill_producer_handle = gapfiller::spawn_gapfill_producer(
            args,
            &block_tip,
            gapfill_tx,
            gapfill_shutdown_tx.clone(),
            Arc::clone(&shared_state),
            Arc::new(node_client.clone()),
            Arc::clone(&source),
        )
        .await?;

        // wait for gapfill to complete
        async {
            tokio::try_join!(gapfill_producer_handle, gapfill_consumer_handle)?;
            Ok::<(), eyre::Report>(())
        }
        .await?;

        let latest_block_number = shared_state.lock().await.get_current_block_number();
        tracing::info!("Backfill finished at block number: {latest_block_number:?}");

        // 2. Run collect elements live async (live-watcher)
        let (tx, rx) = mpsc::channel::<Result<Events<T>>>(channel_size(args));

        let (shutdown_tx, _) = broadcast::channel::<()>(1);

        let consumer_handle = subscriber::spawn_event_consumer(
            rx,
            shutdown_tx.clone(),
            Arc::new(checkpointer.clone()),
            Arc::clone(&sink),
        )
        .await;

        let producer_handle = publisher::spawn_event_producer(
            args,
            tx,
            shutdown_tx.clone(),
            Arc::clone(&shared_state),
            Arc::new(node_client.clone()),
            Arc::clone(&source),
        )
        .await?;

        Ok(Self { shutdown_tx, consumer_handle, producer_handle })
    }

    // Send shutdown signal and wait for both producer and consumer to finish
    pub async fn shutdown(self) {
        let _ = self.shutdown_tx.send(());
        let _ = self.producer_handle.await;
        let _ = self.consumer_handle.await;
    }
}
