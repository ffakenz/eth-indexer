use std::{error::Error, future::Future, pin::Pin, sync::Arc};
use tokio::sync::{broadcast, mpsc};

pub type ConsumerCallback<T> =
    dyn Fn(T) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync + 'static;

pub struct Consumer<T: 'static> {
    consumer_callback: Arc<ConsumerCallback<T>>,
    rx: mpsc::Receiver<T>,
    shutdown_tx: broadcast::Sender<()>,
}

impl<T> Consumer<T> {
    pub fn new(
        consumer_callback: Arc<ConsumerCallback<T>>,
        rx: mpsc::Receiver<T>,
        shutdown_tx: broadcast::Sender<()>,
    ) -> Self {
        Self { consumer_callback, rx, shutdown_tx }
    }
}

impl<T: Send + Sync + 'static> Consumer<T> {
    pub fn spawn<F, Fut>(
        rx: mpsc::Receiver<T>,
        shutdown_tx: broadcast::Sender<()>,
        consumer_callback_factory: F,
    ) -> tokio::task::JoinHandle<()>
    where
        F: Fn(T) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let consumer_callback: Arc<ConsumerCallback<T>> =
            Arc::new(move |msg: T| Box::pin(consumer_callback_factory(msg)));

        let mut consumer = Consumer::new(consumer_callback, rx, shutdown_tx);

        tokio::spawn(async move {
            if let Err(e) = consumer.run().await {
                eprintln!("Consumer failed: {e:?}");
            }
        })
    }
}

impl<T> Consumer<T> {
    pub async fn run(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut shutdown_rx = self.shutdown_tx.subscribe();

        loop {
            tokio::select! {
                // Receive message from the channel
                maybe_msg = self.rx.recv() => {
                    match maybe_msg {
                        Some(data) => {
                            // Execute the callback
                            let consumer_callback = Arc::clone(&self.consumer_callback);
                            consumer_callback(data).await;
                        }
                        None => {
                            // Channel closed
                            break
                        },
                    }
                }
                _ = shutdown_rx.recv() => {
                    // Shutdown signal received
                    break
                }
            }
        }

        Ok(())
    }
}
