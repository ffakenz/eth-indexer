use crate::args::Args;
use crate::live::source::filter::StreamFilter;
use crate::live::source::handle::{Source, SourceInput};
use crate::live::state::event::Events;
use crate::live::state::logic::State;
use crate::live::state::outcome::Outcome;
use chain::rpc::NodeClient;
use eyre::{Result, eyre};
use futures_util::StreamExt;
use std::fmt::Debug;
use std::sync::Arc;
use sync::producer::Producer;
use tokio::sync::{Mutex, broadcast, mpsc};

pub async fn spawn_event_producer<E, T>(
    args: &Args,
    state: State,
    tx: mpsc::Sender<Result<Events<T>>>,
    shutdown_tx: broadcast::Sender<()>,
    node_client: Arc<NodeClient>,
    source: Arc<dyn Source<Item = E>>,
) -> Result<tokio::task::JoinHandle<()>>
where
    E: SourceInput + TryInto<T> + Clone + Debug + Send + Sync + 'static,
    <E as TryInto<T>>::Error: Debug + Send + Sync + 'static,
    T: Outcome + TryFrom<E> + Send + Sync + 'static,
{
    let checkpoint_interval = args.checkpoint_interval;

    let stream_filter = StreamFilter {
        addresses: args.addresses.clone(),
        event: args.event.clone(),
        from_block_number: state.get_next_checkpoint_block_number().into(),
        poll_interval: args.poll_interval,
    };
    let inputs_stream = source.stream(stream_filter).await?;

    // Wrap in a Arc + Mutex for interior mutability.
    // * Arc, allows sharing across async tasks/closures.
    // * Mutex, gives async mutable access:
    let shared_inputs_stream = Arc::new(Mutex::new(inputs_stream));
    let shared_state = Arc::new(Mutex::new(state));

    // A closure that returns a future
    let producer_callback = move || {
        let inputs_stream_for_producer = Arc::clone(&shared_inputs_stream);
        let node_client_for_producer = Arc::clone(&node_client);
        let state_for_producer = Arc::clone(&shared_state);
        async move {
            match inputs_stream_for_producer.lock().await.next().await {
                Some(input) => {
                    tracing::info!("Publisher rolling forward: {input:?}");
                    state_for_producer
                        .lock()
                        .await
                        .on_roll_forward(
                            input,
                            checkpoint_interval,
                            node_client_for_producer.as_ref(),
                        )
                        .await
                }
                None => {
                    tracing::error!("Stream ended");
                    Err(eyre!("Stream ended"))
                }
            }
        }
    };

    // Spawn producer: produces received from logs stream and sends them to tx (consumer)
    Ok(Producer::spawn(tx, shutdown_tx, producer_callback))
}
