use crate::{
    checkpointer::Checkpointer,
    live::{
        sink::handle::Sink,
        state::event::{Event, Events},
    },
};
use eyre::Result;
use std::fmt::Debug;
use std::sync::Arc;
use sync::consumer::Consumer;
use tokio::sync::{broadcast, mpsc};

pub async fn consume_event_outcome<T: Debug>(
    event: Event<T>,
    checkpointer: &Checkpointer,
    sink: &dyn Sink<Item = T>,
) -> Result<()> {
    match event {
        Event::Skip => {
            tracing::info!("Consumer skipped consumed: {event:?}");
            Ok(())
        }
        Event::Checkpoint(block) => {
            tracing::info!("Consumer consumed checkpoint: {block:?}");
            let checkpoint = block.as_ref();
            checkpointer.checkpoint(&checkpoint.into()).await
        }
        Event::Element(e) => {
            tracing::info!("Consumer consumed element: {e:?}");
            sink.process(&e).await
        }
        Event::Many(events) => {
            let number_of_evens = events.len();
            tracing::info!("Consumer consumed {number_of_evens:?} many elements");
            if events.is_empty() {
                Ok(())
            } else if number_of_evens == 1 {
                let e = events.first().unwrap();
                sink.process(e).await
            } else {
                sink.process_batch(&events).await
            }
        }
    }
}

pub async fn spawn_event_consumer<T>(
    rx: mpsc::Receiver<Result<Events<T>>>,
    shutdown_tx: broadcast::Sender<()>,
    checkpointer: Arc<Checkpointer>,
    sink: Arc<dyn Sink<Item = T>>,
) -> tokio::task::JoinHandle<()>
where
    T: Debug + Send + Sync + 'static,
{
    // A closure that returns a future.
    let shutdown_tx_cloned = shutdown_tx.clone();
    let consumer_callback = move |consumed_events: Result<Events<T>>| {
        let checkpointer_for_consumer: Arc<Checkpointer> = Arc::clone(&checkpointer);
        let sink_for_consumer: Arc<dyn Sink<Item = T>> = Arc::clone(&sink);
        let shutdown_tx_for_consumer = shutdown_tx_cloned.clone();
        async move {
            match consumed_events {
                Err(e) => {
                    tracing::error!("Consumer received failed signal from Producer: {e:?}");
                    // stop signal
                    let _ = shutdown_tx_for_consumer.send(());
                }
                Ok(Events(events)) => {
                    for event in events {
                        match consume_event_outcome(
                            event,
                            checkpointer_for_consumer.as_ref(),
                            sink_for_consumer.as_ref(),
                        )
                        .await
                        {
                            Ok(success) => success,
                            Err(e) => {
                                tracing::error!("Consumer failed: {e:?}");
                                // stop signal
                                let _ = shutdown_tx_for_consumer.send(());
                            }
                        }
                    }
                }
            }
        }
    };

    // Spawn consumer: consumes logs from rx (producer)
    Consumer::spawn(rx, shutdown_tx.clone(), consumer_callback)
}
