use eyre::{Result, eyre};
use store::checkpoint::model::Checkpoint;
use store::checkpoint::store::Store as CheckpointStore;

#[derive(Clone)]
pub struct Checkpointer {
    store: CheckpointStore,
}

impl Checkpointer {
    pub fn new(store: CheckpointStore) -> Self {
        Self { store }
    }

    pub async fn get_last_checkpoint(&self) -> Result<Option<Checkpoint>> {
        self.store.get_last_checkpoint().await.map_err(|e| eyre!(e))
    }

    pub async fn checkpoint(&self, checkpoint: &Checkpoint) -> Result<()> {
        match self.store.insert_checkpoint(checkpoint).await {
            Ok(_) => {
                tracing::info!("Checkpoint saved {checkpoint:?}");
                Ok(())
            }
            Err(e) => {
                tracing::error!("Checkpointer failed on [insert_checkpoint]: {e:?}");
                Err(eyre!(e))
            }
        }
    }
}
