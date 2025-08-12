use crate::client::Client;
use crate::model::{Checkpoint, Transfer};
use alloy::primitives::{BlockHash, BlockNumber};
use eyre::Result;

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

    pub async fn insert_transfer(&self, log: &Transfer) -> Result<()> {
        let query = r#"
            INSERT INTO transfers (block_number, block_hash, transaction_hash, log_index, data)
            VALUES (?, ?, ?, ?, ?)
            "#;

        sqlx::query(query)
            .bind(log.block_number)
            .bind(&log.block_hash[..])
            .bind(&log.transaction_hash[..])
            .bind(log.log_index)
            .bind(&log.data)
            .execute(self.client.pool())
            .await?;
        Ok(())
    }

    pub async fn get_transfers_between_block_numbers(
        &self,
        from_block: BlockNumber,
        to_block: BlockNumber,
    ) -> Result<Vec<Transfer>> {
        let query = r#"
            SELECT block_number, block_hash, transaction_hash, log_index, data
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

    pub async fn get_transfers_between_block_hashes(
        &self,
        from_hash: BlockHash,
        to_hash: BlockHash,
    ) -> Result<Vec<Transfer>> {
        let query = r#"
            SELECT block_number, block_hash, transaction_hash, log_index, data
            FROM transfers
            WHERE block_hash >= ? AND block_hash <= ?
            ORDER BY block_number ASC, log_index ASC
            "#;
        let logs = sqlx::query_as(query)
            .bind(&from_hash[..])
            .bind(&to_hash[..])
            .fetch_all(self.client.pool())
            .await?;

        Ok(logs)
    }

    // ---------------------------
    // CHECKPOINTS
    // ---------------------------

    pub async fn insert_checkpoint(
        &self,
        block_number: BlockNumber,
        block_hash: BlockHash,
    ) -> Result<()> {
        let query = r#"
            INSERT INTO checkpoints (block_number, block_hash)
            VALUES (?, ?)
            "#;
        sqlx::query(query)
            .bind(block_number as i64)
            .bind(&block_hash[..])
            .execute(self.client.pool())
            .await?;
        Ok(())
    }

    pub async fn get_last_checkpoint(&self) -> Result<Option<Checkpoint>> {
        let query = r#"
            SELECT block_number, block_hash
            FROM checkpoints
            ORDER BY id DESC
            LIMIT 1
            "#;
        let checkpoint = sqlx::query_as(query).fetch_optional(self.client.pool()).await?;

        Ok(checkpoint)
    }

    pub async fn get_checkpoint_by_number(
        &self,
        block_number: BlockNumber,
    ) -> Result<Option<Checkpoint>> {
        let query = r#"
            SELECT block_number, block_hash
            FROM checkpoints
            WHERE block_number = ?
            LIMIT 1
            "#;
        let checkpoint = sqlx::query_as(query)
            .bind(block_number as i64)
            .fetch_optional(self.client.pool())
            .await?;

        Ok(checkpoint)
    }

    pub async fn get_checkpoint_by_hash(
        &self,
        block_hash: BlockHash,
    ) -> Result<Option<Checkpoint>> {
        let query = r#"
            SELECT block_number, block_hash
            FROM checkpoints
            WHERE block_hash = ?
            LIMIT 1
            "#;
        let checkpoint =
            sqlx::query_as(query).bind(&block_hash[..]).fetch_optional(self.client.pool()).await?;

        Ok(checkpoint)
    }
}
