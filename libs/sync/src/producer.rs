use std::{error::Error, future::Future, pin::Pin, sync::Arc};
use tokio::sync::{broadcast, mpsc};

pub type ProducerCallback<T> =
    dyn Fn() -> Pin<Box<dyn Future<Output = T> + Send>> + Send + Sync + 'static;

pub struct Producer<T: 'static> {
    producer_callback: Arc<ProducerCallback<T>>,
    tx: mpsc::Sender<T>,
    shutdown_tx: broadcast::Sender<()>,
}

impl<T> Producer<T> {
    pub fn new(
        producer_callback: Arc<ProducerCallback<T>>,
        tx: mpsc::Sender<T>,
        shutdown_tx: broadcast::Sender<()>,
    ) -> Self {
        Self { producer_callback, tx, shutdown_tx }
    }
}

impl<T: Send + Sync + 'static> Producer<T> {
    pub fn spawn<F, Fut>(
        tx: mpsc::Sender<T>,
        shutdown_tx: broadcast::Sender<()>,
        producer_callback_factory: F,
    ) -> tokio::task::JoinHandle<()>
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = T> + Send + 'static,
    {
        let producer_callback: Arc<ProducerCallback<T>> =
            Arc::new(move || Box::pin(producer_callback_factory()));

        let producer = Producer::new(producer_callback, tx, shutdown_tx);

        tokio::spawn(async move {
            if let Err(e) = producer.run().await {
                eprintln!("Producer failed: {e:?}");
            }
        })
    }
}

impl<T> Producer<T> {
    pub async fn run(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut shutdown_rx = self.shutdown_tx.subscribe();

        loop {
            tokio::select! {
                message = async {
                    // Execute the callback
                    let callback = Arc::clone(&self.producer_callback);
                    callback().await
                } => {
                    // Send message to the channel
                    if self.tx.send(message).await.is_err() {
                        // The receiver dropped
                        return Ok(());
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
