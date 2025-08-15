use crate::args::Args;
use crate::pubsub::event::Event;
use crate::source::handle::{Source, SourceInput, StreamFilter};
use chain::rpc::NodeClient;
use eyre::eyre;
use eyre::{Report, Result};
use futures_util::StreamExt;
use std::fmt::Debug;
use std::sync::Arc;
use sync::producer::Producer;
use tokio::sync::{Mutex, broadcast, mpsc};

#[derive(Debug)]
struct ProducerState {
    event_counter: u64,
    next_checkpoint_block_number: u64,
}

impl ProducerState {
    fn increment_event_counter(&mut self) {
        self.event_counter += 1;
    }

    fn reset_event_counter(&mut self) {
        self.event_counter = 0;
    }

    fn set_next_checkpoint_block_number(&mut self, block_number: u64) {
        self.next_checkpoint_block_number = block_number;
    }

    // Every N logs, produce a checkpoint event (skip first iteration)
    fn checkpoint_decision(&self, interval: u64) -> (bool, u64) {
        (
            self.event_counter > 0 && self.event_counter == interval,
            self.next_checkpoint_block_number,
        )
    }
}

async fn with_state<S, R>(state: &Arc<Mutex<S>>, f: impl FnOnce(&mut S) -> R) -> R {
    let mut locked = state.lock().await;
    f(&mut locked)
}

pub async fn spawn_event_producer<E, T>(
    args: &Args,
    tx: mpsc::Sender<Result<Event<T>, Report>>,
    shutdown_tx: broadcast::Sender<()>,
    next_checkpoint_block_number: u64,
    source: Arc<dyn Source<Item = E>>,
    node_client: Arc<NodeClient>,
) -> Result<tokio::task::JoinHandle<()>>
where
    E: SourceInput + Clone + Debug + Send + Sync + 'static,
    T: TryFrom<E> + Send + Sync + 'static,
{
    let stream_filter = StreamFilter {
        addresses: args.addresses.clone(),
        event: args.event.clone(),
        from_block_number: next_checkpoint_block_number.into(),
        poll_interval: args.poll_interval,
    };
    let inputs_stream = source.stream(stream_filter).await?;

    // Wrap in a Arc + Mutex for interior mutability.
    // * Arc, allows sharing across async tasks/closures.
    // * Mutex, gives async mutable access:
    let shared_inputs_stream = Arc::new(Mutex::new(inputs_stream));
    let state =
        Arc::new(Mutex::new(ProducerState { event_counter: 0, next_checkpoint_block_number }));

    let checkpoint_interval = args.checkpoint_interval;

    // A closure that returns a future
    let producer_callback = move || {
        let inputs_stream_for_producer = Arc::clone(&shared_inputs_stream);
        let node_client_for_producer = Arc::clone(&node_client);
        let state_for_producer = Arc::clone(&state);
        async move {
            let (do_checkpoint, checkpoint_block_number) =
                state_for_producer.lock().await.checkpoint_decision(checkpoint_interval);

            if do_checkpoint {
                let checkpoint_block = node_client_for_producer
                    .get_block_by_number(checkpoint_block_number)
                    .await?
                    // REVIEW! shall we skip instead?
                    .ok_or_else(|| {
                        eyre!("Block not found for checkpoint: {}", checkpoint_block_number)
                    })?;

                with_state(&state_for_producer, |s| {
                    s.reset_event_counter();
                    Ok(Event::Checkpoint(Box::new(checkpoint_block)))
                })
                .await
            } else {
                // Produce a input event
                match inputs_stream_for_producer.lock().await.next().await {
                    Some(input) => {
                        if let Some(log_block_number) = input.block_number() {
                            // XXX: input mapper
                            let input_cloned = input.clone();
                            match input.try_into().map_err(|_| {
                                eyre!("Failed to convert consumed input: {input_cloned:?}")
                            }) {
                                Err(_) => {
                                    with_state(&state_for_producer, |s| {
                                        s.increment_event_counter();
                                        Ok(Event::Skip)
                                    })
                                    .await
                                }
                                Ok(e) => {
                                    with_state(&state_for_producer, |s| {
                                        s.set_next_checkpoint_block_number(log_block_number);
                                        s.increment_event_counter();
                                        Ok(Event::Element(Box::new(e)))
                                    })
                                    .await
                                }
                            }
                        } else {
                            with_state(&state_for_producer, |s| {
                                s.increment_event_counter();
                                Ok(Event::Skip)
                            })
                            .await
                        }
                    }
                    None => Err(Report::msg("Stream ended")),
                }
            }
        }
    };

    // Spawn producer: produces received from logs stream and sends them to tx (consumer)
    Ok(Producer::spawn(tx, shutdown_tx, producer_callback))
}
