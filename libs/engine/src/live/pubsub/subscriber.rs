use crate::{
    checkpointer::Checkpointer,
    live::{sink::handle::Sink, state::event::Event},
};
use eyre::Result;
use std::fmt::Debug;
use std::sync::Arc;
use sync::consumer::Consumer;
use tokio::sync::{broadcast, mpsc};

pub async fn spawn_event_consumer<T>(
    rx: mpsc::Receiver<Result<Event<T>>>,
    shutdown_tx: broadcast::Sender<()>,
    checkpointer: Arc<Checkpointer>,
    sink: Arc<dyn Sink<Item = T>>,
) -> tokio::task::JoinHandle<()>
where
    T: Debug + Send + Sync + 'static,
{
    // A closure that returns a future.
    let shutdown_tx_cloned = shutdown_tx.clone();
    let consumer_callback = move |consumed_event: Result<Event<T>>| {
        let checkpointer_for_consumer: Arc<Checkpointer> = Arc::clone(&checkpointer);
        let sink_for_consumer: Arc<dyn Sink<Item = T>> = Arc::clone(&sink);
        let shutdown_tx_for_consumerr = shutdown_tx_cloned.clone();
        async move {
            match consumed_event {
                Err(e) => {
                    tracing::error!("Consumer received failed signal from Producer: {e:?}");
                    // stop signal
                    let _ = shutdown_tx_for_consumerr.send(());
                }
                Ok(event @ Event::Skip) => {
                    tracing::info!("Consumer skipped consumed: {event:?}");
                }
                Ok(Event::Checkpoint(ref block)) => {
                    tracing::info!("Consumer consumed checkpoint: {block:?}");
                    match checkpointer_for_consumer.checkpoint(block.as_ref()).await {
                        Ok(success) => success,
                        Err(e) => {
                            tracing::error!("Consumer failed on [save_checkpoint]: {e:?}");
                            // stop signal
                            let _ = shutdown_tx_for_consumerr.send(());
                        }
                    }
                }
                Ok(Event::Element(ref e)) => {
                    tracing::info!("Consumer consumed element: {e:?}");
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
