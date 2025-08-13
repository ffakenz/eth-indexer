use alloy::{hex, rpc::types::Log};
use eyre::{Result, eyre};
use sqlx::Error;
use store::transfer::{model::Transfer, store::Store};

#[async_trait::async_trait]
pub trait Processor<T>: Send + Sync {
    async fn process_log(&self, log: &Log) -> Result<T>;
}

pub struct TransferProcessor {
    pub store: Store,
}

#[async_trait::async_trait]
impl Processor<Option<Transfer>> for TransferProcessor {
    async fn process_log(&self, log: &Log) -> Result<Option<Transfer>> {
        let transfer: Transfer = log.try_into()?;
        match self.store.insert_transfer(&transfer).await {
            Ok(_) => {
                println!("Processed: {transfer:?}");
                Ok(Some(transfer))
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
                        Ok(None)
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
