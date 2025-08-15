use crate::pubsub::event::Event;
use crate::{checkpointer, sink::handle::Sink};
use eyre::{Report, Result};
use std::fmt::Debug;
use std::sync::Arc;
use store::checkpoint::store::Store as CheckpointStore;
use sync::consumer::Consumer;
use tokio::sync::{broadcast, mpsc};

pub async fn spawn_event_consumer<T>(
    rx: mpsc::Receiver<Result<Event<T>, Report>>,
    shutdown_tx: broadcast::Sender<()>,
    checkpoint_store: Arc<CheckpointStore>,
    sink: Arc<dyn Sink<Item = T>>,
) -> tokio::task::JoinHandle<()>
where
    T: Debug + Send + Sync + 'static,
{
    // A closure that returns a future.
    let shutdown_tx_cloned = shutdown_tx.clone();
    let consumer_callback = move |consumed_event: Result<Event<T>>| {
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
                Ok(Event::Element(ref e)) => {
                    println!("Consumer consumed element: {e:?}");
                    match sink_for_consumer.process(e).await {
                        Ok(success) => success,
                        Err(_) => {
                            // stop signal
                            let _ = shutdown_tx_for_consumerr.send(());
                        }
                    }
                }
            }
        }
    };

    // Spawn consumer: consumes logs from rx (producer)
    Consumer::spawn(rx, shutdown_tx.clone(), consumer_callback)
}
