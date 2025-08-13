use crate::client::Client;
use crate::transfer::model::Transfer;
use alloy::primitives::BlockNumber;
use eyre::Result;
use sqlx::Error;

pub struct Store {
    client: Client,
}

impl Store {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    // ---------------------------
    // TRANSFER LOGS
    // ---------------------------

    pub async fn insert_transfer(&self, log: &Transfer) -> Result<(), Error> {
        let query = r#"
            INSERT INTO transfers (
                block_number, block_hash, transaction_hash, log_index,
                contract_address, from_address, to_address, amount
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#;

        sqlx::query(query)
            .bind(log.block_number)
            .bind(&log.block_hash)
            .bind(&log.transaction_hash)
            .bind(log.log_index)
            .bind(&log.contract_address)
            .bind(&log.from_address)
            .bind(&log.to_address)
            .bind(&log.amount)
            .execute(self.client.pool())
            .await?;
        Ok(())
    }

    pub async fn get_transfers_from_block_number(
        &self,
        from_block_number: BlockNumber,
    ) -> Result<Vec<Transfer>, Error> {
        let query = r#"
            SELECT
                block_number, block_hash, transaction_hash, log_index,
                contract_address, from_address, to_address, amount
            FROM transfers
            WHERE block_number >= ?
            ORDER BY block_number ASC, log_index ASC
            "#;
        let logs = sqlx::query_as(query)
            .bind(from_block_number as i64)
            .fetch_all(self.client.pool())
            .await?;

        Ok(logs)
    }

    pub async fn get_transfers_between_block_numbers(
        &self,
        from_block: BlockNumber,
        to_block: BlockNumber,
    ) -> Result<Vec<Transfer>, Error> {
        let query = r#"
            SELECT
                block_number, block_hash, transaction_hash, log_index,
                contract_address, from_address, to_address, amount
            FROM transfers
            WHERE block_number BETWEEN ? AND ?
            ORDER BY block_number ASC, log_index ASC
            "#;
        let logs = sqlx::query_as(query)
            .bind(from_block as i64)
            .bind(to_block as i64)
            .fetch_all(self.client.pool())
            .await?;

        Ok(logs)
    }
}
