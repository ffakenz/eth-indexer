use alloy::hex;
use eyre::{Result, eyre};
use sqlx::Error;
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
                println!("Processed: {transfer:?}");
                Ok(())
            }
            Err(e) => {
                if let Error::Database(db_err) = &e {
                    if db_err.message().contains(
                        "UNIQUE constraint failed: transfers.transaction_hash, transfers.log_index",
                    ) {
                        println!(
                            "Duplicate transfer ignored: tx_hash={}, log_index={}",
                            hex::encode(&transfer.transaction_hash),
                            transfer.log_index
                        );
                        Ok(())
                    } else {
                        eprintln!("Processor failed on [insert_transfer]: {e:?}");
                        Err(eyre!(e))
                    }
                } else {
                    eprintln!("Processor fatal error on [insert_transfer]: {e:?}");
                    Err(eyre!(e))
                }
            }
        }
    }
}
