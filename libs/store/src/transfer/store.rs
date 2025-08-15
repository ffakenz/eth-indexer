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
            INSERT OR IGNORE INTO transfers (
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

    /// Inserts multiple transfers in batches, respecting SQLite's max variable limit.
    pub async fn insert_transfers_batch(&self, transfers: &[Transfer]) -> Result<(), Error> {
        if transfers.is_empty() {
            return Ok(());
        }

        // SQLite variable limit = 999 by default
        const COLS: usize = 8;
        const SQLITE_MAX_VARIABLES: usize = 999;
        let max_rows_per_batch = SQLITE_MAX_VARIABLES / COLS;

        let mut start = 0;
        while start < transfers.len() {
            let end = (start + max_rows_per_batch).min(transfers.len());
            let batch = &transfers[start..end];

            let values_placeholders =
                (0..batch.len()).map(|_| "(?, ?, ?, ?, ?, ?, ?, ?)").collect::<Vec<_>>().join(", ");

            // SQLite skips rows that violate the constraint, keeps the rest.
            let mut query = String::from(
                "INSERT OR IGNORE INTO transfers (
                    block_number, block_hash, transaction_hash, log_index,
                    contract_address, from_address, to_address, amount
                ) VALUES ",
            );
            query.push_str(&values_placeholders);

            let mut q = sqlx::query(&query);
            for log in batch {
                q = q
                    .bind(log.block_number)
                    .bind(&log.block_hash)
                    .bind(&log.transaction_hash)
                    .bind(log.log_index)
                    .bind(&log.contract_address)
                    .bind(&log.from_address)
                    .bind(&log.to_address)
                    .bind(&log.amount);
            }

            // Wrap in transaction for speed + atomicity
            let mut tx = self.client.pool().begin().await?;
            q.execute(&mut *tx).await?;
            tx.commit().await?;

            start = end;
        }

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
