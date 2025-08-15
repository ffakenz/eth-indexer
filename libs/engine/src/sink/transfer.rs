use eyre::{Result, eyre};
use store::transfer::{model::Transfer, store::Store};

use crate::sink::handle::Sink;

pub struct TransferSink {
    pub store: Store,
}

#[async_trait::async_trait]
impl Sink for TransferSink {
    type Item = Transfer;

    async fn process(&self, transfer: &Transfer) -> Result<()> {
        match self.store.insert_transfer(transfer).await {
            Ok(_) => {
                tracing::info!("Processed: {transfer:?}");
                Ok(())
            }
            Err(e) => {
                tracing::error!("Processor failed on [insert_transfer]: {e:?}");
                Err(eyre!(e))
            }
        }
    }

    async fn process_batch(&self, transfers: &[Transfer]) -> Result<()> {
        match self.store.insert_transfers_batch(transfers).await {
            Ok(_) => {
                let nbr_of_rows = transfers.len();
                tracing::info!("Processed batch: {nbr_of_rows:?}");
                Ok(())
            }
            Err(e) => {
                tracing::error!("Processor failed on [insert_transfers_batch]: {e:?}");
                Err(eyre!(e))
            }
        }
    }
}
