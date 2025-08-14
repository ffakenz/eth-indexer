use crate::pubsub::event::Event;
use crate::source::handle::SourceInput;
use crate::{checkpointer, sink::handle::Sink};
use eyre::{Report, Result, eyre};
use std::fmt;
use std::sync::Arc;
use store::checkpoint::store::Store as CheckpointStore;
use sync::consumer::Consumer;
use tokio::sync::{broadcast, mpsc};

pub async fn spawn_event_consumer<E, T>(
    rx: mpsc::Receiver<Result<Event<E>, Report>>,
    shutdown_tx: broadcast::Sender<()>,
    checkpoint_store: Arc<CheckpointStore>,
    sink: Arc<dyn Sink<Item = T>>,
) -> tokio::task::JoinHandle<()>
where
    E: SourceInput + fmt::Debug + Clone + Send + Sync + 'static,
    T: TryFrom<E> + Send + Sync + 'static,
{
    // A closure that returns a future.
    let shutdown_tx_cloned = shutdown_tx.clone();
    let consumer_callback = move |consumed_event: Result<Event<E>>| {
        let checkpoint_store_for_consumer: Arc<CheckpointStore> = Arc::clone(&checkpoint_store);
        let sink_for_consumer: Arc<dyn Sink<Item = T>> = Arc::clone(&sink);
        let shutdown_tx_for_consumerr = shutdown_tx_cloned.clone();
        async move {
            match consumed_event {
                Err(e) => {
                    eprintln!("Consumer received failed signal from Producer: {e:?}");
                    // stop signal
                    let _ = shutdown_tx_for_consumerr.send(());
                }
                Ok(event @ Event::Skip) => {
                    println!("Consumer skipped consumed: {event:?}");
                }
                Ok(Event::Checkpoint(ref block)) => {
                    println!("Consumer consumed checkpoint: {block:?}");
                    match checkpointer::save_checkpoint(block, &checkpoint_store_for_consumer).await
                    {
                        Ok(success) => success,
                        Err(e) => {
                            eprintln!("Consumer failed on [save_checkpoint]: {e:?}");
                            // stop signal
                            let _ = shutdown_tx_for_consumerr.send(());
                        }
                    }
                }
                Ok(Event::Input(ref input)) => {
                    println!("Consumer consumed input: {input:?}");
                    // XXX: input mapper
                    let input_cloned = input.as_ref().clone();
                    match input_cloned
                        .try_into()
                        .map_err(|_| eyre!("Failed to convert consumed input: {input:?}"))
                    {
                        Err(_) => {
                            // stop signal
                            let _ = shutdown_tx_for_consumerr.send(());
                        }
                        Ok(element) => match sink_for_consumer.process(&element).await {
                            Ok(_) => (),
                            Err(_) => {
                                // stop signal
                                let _ = shutdown_tx_for_consumerr.send(());
                            }
                        },
                    }
                }
            }
        }
    };

    // Spawn consumer: consumes logs from rx (producer)
    Consumer::spawn(rx, shutdown_tx.clone(), consumer_callback)
}
