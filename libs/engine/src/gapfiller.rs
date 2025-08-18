use crate::args::Args;
use crate::live::source::filter::ChunkFilter;
use crate::live::source::handle::{Source, SourceInput};
use crate::live::state::event::Events;
use crate::live::state::logic::State;
use crate::live::state::outcome::Outcome;
use alloy::rpc::types::Block;
use chain::rpc::NodeClient;
use eyre::{Result, eyre};
use std::fmt::Debug;
use std::sync::Arc;
use sync::producer::Producer;
use tokio::sync::{Mutex, broadcast, mpsc};

pub async fn spawn_gapfill_producer<E, T>(
    args: &Args,
    block_tip: &Block,
    tx: mpsc::Sender<Result<Events<T>>>,
    shutdown_tx: broadcast::Sender<()>,
    shared_state: Arc<Mutex<State>>,
    node_client: Arc<NodeClient>,
    source: Arc<dyn Source<Item = E>>,
) -> Result<tokio::task::JoinHandle<()>>
where
    E: SourceInput + TryInto<T> + Clone + Debug + Send + Sync + 'static,
    <E as TryInto<T>>::Error: Debug + Send + Sync + 'static,
    T: Outcome + TryFrom<E> + Send + Sync + 'static,
{
    let checkpoint_interval = args.backfill_checkpoint_interval.unwrap_or(args.checkpoint_interval);
    let latest_block_number = block_tip.number();

    let shared_addresses = args.addresses.clone();
    let shared_event = args.event.clone();

    let producer_callback = move || {
        let state_for_producer = Arc::clone(&shared_state);
        let node_client_for_producer = Arc::clone(&node_client);
        let source_for_producer = Arc::clone(&source);

        let address_for_producer = shared_addresses.clone();
        let event_for_producer = shared_event.clone();

        async move {
            let from_block_number = state_for_producer.lock().await.get_current_block_number() + 1;

            let chunk_block_number =
                from_block_number.saturating_add(checkpoint_interval - 1).min(latest_block_number);

            let tip_exceeded = from_block_number > latest_block_number;
            let done = from_block_number == chunk_block_number;
            if tip_exceeded || done {
                tracing::info!("Gapfill ended");
                return Err(eyre!("Gapfill ended"));
            }

            let chunk_filter = ChunkFilter {
                addresses: address_for_producer,
                event: event_for_producer,
                from_block_number: from_block_number.into(),
                to_block_number: chunk_block_number.into(),
            };

            let source_inputs = source_for_producer.chunk(chunk_filter).await?;
            state_for_producer
                .lock()
                .await
                .roll_forward_batch(
                    source_inputs,
                    checkpoint_interval,
                    node_client_for_producer.as_ref(),
                )
                .await
        }
    };

    // Spawn producer: produces received inputs from chunk batch
    // and sends them rolled forward batch events to tx (consumer)
    Ok(Producer::spawn(tx, shutdown_tx, producer_callback))
}
