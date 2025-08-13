use crate::pubsub::event::Event;
use crate::{checkpointer, processor::handle::Processor};
use alloy::rpc::types::Log;
use eyre::{Report, Result};
use std::sync::Arc;
use store::checkpoint::store::Store as CheckpointStore;
use sync::consumer::Consumer;
use tokio::sync::{broadcast, mpsc};

pub async fn spawn_event_consumer<T: Send + Sync + 'static>(
    rx: mpsc::Receiver<Result<Event, Report>>,
    shutdown_tx: broadcast::Sender<()>,
    checkpoint_store: Arc<CheckpointStore>,
    processor: Arc<dyn Processor<Log, T>>,
) -> tokio::task::JoinHandle<()> {
    // A closure that returns a future.
    let shutdown_tx_cloned = shutdown_tx.clone();
    let consumer_callback = move |consumed_event: Result<Event>| {
        let checkpoint_store_for_consumer: Arc<CheckpointStore> = Arc::clone(&checkpoint_store);
        let processor_for_consumer: Arc<dyn Processor<Log, T>> = Arc::clone(&processor);
        let shutdown_tx_for_consumerr = shutdown_tx_cloned.clone();
        async move {
            match &consumed_event {
                Err(e) => {
                    eprintln!("Consumer received failed signal from Producer: {e:?}");
                    // stop signal
                    let _ = shutdown_tx_for_consumerr.send(());
                }
                Ok(event @ Event::Skip) => {
                    println!("Consumer skipped consumed: {event:?}");
                }
                Ok(event @ Event::Checkpoint(block)) => {
                    println!("Consumer consumed: {event:?}");
                    match checkpointer::save_checkpoint(
                        block.as_ref(),
                        &checkpoint_store_for_consumer,
                    )
                    .await
                    {
                        Ok(success) => success,
                        Err(e) => {
                            eprintln!("Consumer failed on [save_checkpoint]: {e:?}");
                            // stop signal
                            let _ = shutdown_tx_for_consumerr.send(());
                        }
                    }
                }
                Ok(event @ Event::Log(log)) => {
                    println!("Consumer consumed: {event:?}");
                    match log.as_ref().block_number {
                        // NOTE: This log comes from a pending tx that has not yet been mined.
                        // Pending logs are re-emitted (with the same tx hash and log index)
                        // once the tx is included in a block, at which point `block_number` will be set.
                        // We skip it (ignore) now and process it only after confirmation.
                        None => (),
                        // the tx that emitted this log has been mined into a block
                        Some(_) => {
                            match processor_for_consumer.process_log(log.as_ref()).await {
                                Ok(_) => (),
                                Err(_) => {
                                    // stop signal
                                    let _ = shutdown_tx_for_consumerr.send(());
                                }
                            };
                        }
                    }
                }
            }
        }
    };

    // Spawn consumer: consumes logs from rx (producer)
    Consumer::spawn(rx, shutdown_tx.clone(), consumer_callback)
}
