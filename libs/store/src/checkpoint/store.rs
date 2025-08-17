use crate::checkpoint::model::Checkpoint;
use crate::client::Client;
use alloy::primitives::{BlockHash, BlockNumber};
use eyre::Result;
use sqlx::Error;

#[derive(Clone)]
pub struct Store {
    client: Client,
}

impl Store {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    // ---------------------------
    // CHECKPOINTS
    // ---------------------------

    pub async fn insert_checkpoint(&self, checkpoint: &Checkpoint) -> Result<(), Error> {
        let query = r#"
            INSERT OR IGNORE INTO checkpoints (block_number, block_hash, parent_hash)
            VALUES (?, ?, ?)
            "#;
        sqlx::query(query)
            .bind(checkpoint.block_number)
            .bind(&checkpoint.block_hash)
            .bind(&checkpoint.parent_hash)
            .execute(self.client.pool())
            .await?;
        Ok(())
    }

    pub async fn get_last_checkpoint(&self) -> Result<Option<Checkpoint>, Error> {
        let query = r#"
            SELECT block_number, block_hash, parent_hash
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
    ) -> Result<Option<Checkpoint>, Error> {
        let query = r#"
            SELECT block_number, block_hash, parent_hash
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
    ) -> Result<Option<Checkpoint>, Error> {
        let query = r#"
            SELECT block_number, block_hash, parent_hash
            FROM checkpoints
            WHERE block_hash = ?
            LIMIT 1
            "#;
        let checkpoint =
            sqlx::query_as(query).bind(&block_hash[..]).fetch_optional(self.client.pool()).await?;

        Ok(checkpoint)
    }
}
