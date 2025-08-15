use crate::args::Args;
use crate::source::filter::StreamFilter;
use crate::source::handle::{Source, SourceInput};
use crate::state::event::Event;
use crate::state::logic::State;
use crate::state::outcome::Outcome;
use chain::rpc::NodeClient;
use eyre::{Result, eyre};
use futures_util::StreamExt;
use std::fmt::Debug;
use std::sync::Arc;
use sync::producer::Producer;
use tokio::sync::{Mutex, broadcast, mpsc};

pub async fn with_state<S, R>(state: &Arc<Mutex<S>>, f: impl FnOnce(&mut S) -> R) -> R {
    let mut locked = state.lock().await;
    f(&mut locked)
}

pub async fn spawn_event_producer<E, T>(
    args: &Args,
    state: State,
    tx: mpsc::Sender<Result<Event<T>>>,
    shutdown_tx: broadcast::Sender<()>,
    node_client: Arc<NodeClient>,
    source: Arc<dyn Source<Item = E>>,
) -> Result<tokio::task::JoinHandle<()>>
where
    E: SourceInput + Clone + Debug + Send + Sync + 'static,
    <E as TryInto<T>>::Error: Debug,
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
            let (do_checkpoint, checkpoint_block_number) =
                state_for_producer.lock().await.checkpoint_decision(checkpoint_interval);
            if do_checkpoint {
                let maybe_checkpoint_block =
                    node_client_for_producer.get_block_by_number(checkpoint_block_number).await?;
                with_state(&state_for_producer, |s| s.on_checkpoint(maybe_checkpoint_block)).await
            } else {
                match inputs_stream_for_producer.lock().await.next().await {
                    Some(input) => with_state(&state_for_producer, |s| s.on_input(input)).await,
                    None => Err(eyre!("Stream ended")),
                }
            }
        }
    };

    // Spawn producer: produces received from logs stream and sends them to tx (consumer)
    Ok(Producer::spawn(tx, shutdown_tx, producer_callback))
}
