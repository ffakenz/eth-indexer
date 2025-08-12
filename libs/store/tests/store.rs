#[cfg(test)]
mod tests {
    use alloy::primitives::{B256, BlockHash, BlockNumber};
    use eyre::Result;
    use store::{client::Client, store::Store};

    #[tokio::test]
    async fn test_insert_and_get_last_checkpoint() -> Result<()> {
        let db_url = "sqlite::memory:";
        let client = Client::init(db_url).await?;
        let store = Store::new(client);

        let block_number_1: BlockNumber = 12345;
        let block_hash_1: BlockHash = B256::repeat_byte(0xAB);
        store.insert_checkpoint(block_number_1, block_hash_1).await?;

        let block_number_2: BlockNumber = 12346;
        let block_hash_2: BlockHash = B256::repeat_byte(0xCD);
        store.insert_checkpoint(block_number_2, block_hash_2).await?;

        let last = store.get_last_checkpoint().await?;

        assert!(last.is_some());
        let last = last.unwrap();

        assert!(last.block_number > block_number_1 as i64);
        assert_eq!(last.block_number, block_number_2 as i64);
        assert_eq!(last.block_hash, block_hash_2.as_slice());

        Ok(())
    }
}
