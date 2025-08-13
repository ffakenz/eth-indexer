#[cfg(test)]
mod tests {
    use alloy::primitives::B256;
    use eyre::Result;
    use store::{client::Client, model::Checkpoint, store::Store};

    #[tokio::test]
    async fn test_insert_and_get_last_checkpoint() -> Result<()> {
        let db_url = "sqlite::memory:";
        let client = Client::init(db_url).await?;
        let store = Store::new(client);

        let checkpoint_1 = Checkpoint {
            block_number: 12345,
            block_hash: B256::repeat_byte(0xAB).to_vec(),
            parent_hash: B256::repeat_byte(0xBA).to_vec(),
        };
        store.insert_checkpoint(checkpoint_1.clone()).await?;

        let checkpoint_2 = Checkpoint {
            block_number: 12346,
            block_hash: B256::repeat_byte(0xCD).to_vec(),
            parent_hash: B256::repeat_byte(0xDC).to_vec(),
        };
        store.insert_checkpoint(checkpoint_2.clone()).await?;

        let last_checkpoint = store.get_last_checkpoint().await?;

        assert!(last_checkpoint.is_some());
        let last_checkpoint = last_checkpoint.unwrap();

        assert!(last_checkpoint.block_number > checkpoint_1.block_number);
        assert_eq!(last_checkpoint.block_number, checkpoint_2.block_number);
        assert_eq!(last_checkpoint.block_hash, checkpoint_2.block_hash);
        assert_eq!(last_checkpoint.parent_hash, checkpoint_2.parent_hash);

        Ok(())
    }
}
