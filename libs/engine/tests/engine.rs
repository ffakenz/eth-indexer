#[cfg(test)]
mod tests {
    use alloy::primitives::BlockHash;
    use alloy::rpc::types::{Log, ValueOrArray};
    use engine::engine::Engine;
    use engine::processor::{handle::Processor, transfer::TransferProcessor};
    use eyre::Result;
    use store::checkpoint::store::Store as CheckpointStore;
    use store::client::Client;
    use store::transfer::model::Transfer;
    use store::transfer::store::Store as TransferStore;

    use std::sync::Arc;
    use std::{collections::HashSet, time::Duration};

    use alloy::{
        node_bindings::Anvil,
        primitives::{Address, TxHash, U256},
        signers::local::PrivateKeySigner,
        sol,
    };

    use chain::rpc::NodeClient;

    // Codegen from artifact.
    sol!(
        #[allow(missing_docs)]
        #[sol(rpc)]
        ZamaToken,
        "../../resources/tests/zamatoken/ZamaToken.json"
    );

    #[tokio::test]
    async fn test_polling_transfer_event_logs() -> Result<()> {
        // Init SQLite store
        let db_url = "sqlite::memory:";
        let client = Client::init(db_url).await?;
        let checkpoint_store = Arc::new(CheckpointStore::new(client.clone()));
        let transfer_store = TransferStore::new(client.clone());
        let transfer_processor: Arc<dyn Processor<Log, Option<Transfer>>> =
            Arc::new(TransferProcessor { store: TransferStore::new(client.clone()) });

        // Spin up a local Anvil node.
        // Ensure `anvil` is available in $PATH.
        let anvil = Anvil::new().block_time(1).try_spawn()?;
        let pk: PrivateKeySigner = anvil.keys()[0].clone().into();
        let owner: Address = pk.address();

        // Create a RPC client provider.
        let rpc_url = anvil.endpoint_url();
        let node_client = NodeClient::new(rpc_url.clone(), pk.clone());

        // Deploy the `ZamaToken` contract.
        println!("Deploying contract...");
        println!("Signer: {owner:?}");
        let contract = ZamaToken::deploy(node_client.borrow_provider(), owner).await?;
        println!("Deployed ERC20 contract at: {}", contract.address());
        println!("Owner: {:?}", contract.owner().call().await?);

        // Get two accounts from Anvil, Alice and Bob.
        let accounts = node_client.get_accounts().await?;
        let alice = accounts[1];
        let alice_pk: PrivateKeySigner = anvil.keys()[1].clone().into();
        let alice_client = NodeClient::new(rpc_url.clone(), alice_pk);
        let alice_contract = ZamaToken::new(*contract.address(), alice_client.borrow_provider());
        println!("Alice: {alice:?}");
        let bob = accounts[2];
        println!("Bob: {bob:?}");

        // Fetch latest block after scenario setup
        let start_block = node_client.get_latest_block().await?.unwrap();
        let start_block_hash = start_block.hash();
        println!("Start block: {start_block_hash:?}");

        let mut expected_tx_hashes: HashSet<TxHash> = HashSet::new();

        // Fund Alice
        println!("Funding Alice");
        let pending_mint_tx = contract.mint(alice, U256::from(1000)).send().await?;
        println!("Mint tx hash: {:?}", pending_mint_tx.tx_hash());
        let tx_receipt = pending_mint_tx.get_receipt().await?;
        println!("Mint tx receipt: {tx_receipt:?}");
        expected_tx_hashes.insert(tx_receipt.transaction_hash);

        // Register the balances of Alice and Bob before the transfer.
        let alice_before = contract.balanceOf(alice).call().await?;
        let bob_before = contract.balanceOf(bob).call().await?;

        // Send transfer 1 from Alice -> Bob (before engine startup)
        let amount_1 = U256::from(100);
        alice_contract.approve(pk.address(), amount_1).send().await?.watch().await?;
        let tx_hash_1 = contract.transferFrom(alice, bob, amount_1).send().await?.watch().await?;
        println!("Sent transfer tx: {tx_hash_1}");
        expected_tx_hashes.insert(tx_hash_1);

        // Start the engine
        let args = engine::args::Args {
            addresses: ValueOrArray::Value(*contract.address()),
            event: "Transfer(address,address,uint256)".to_string(),
            from_block: start_block_hash,
            backfill_chunk_size: 2,
            checkpoint_interval: 1,
            poll_interval: Duration::from_millis(100),
        };
        let engine = Engine::start(
            &args,
            &node_client,
            Arc::clone(&checkpoint_store),
            Arc::clone(&transfer_processor),
        )
        .await?;

        // Send transfer 2 from Alice -> Bob (after engine startup)
        let amount_2 = U256::from(100);
        alice_contract.approve(pk.address(), amount_2).send().await?.watch().await?;
        let tx_hash_2 = contract.transferFrom(alice, bob, amount_2).send().await?.watch().await?;
        println!("Sent transfer tx2: {tx_hash_2}");
        expected_tx_hashes.insert(tx_hash_2);

        // Let the engine run for a few iterations
        tokio::time::sleep(Duration::from_secs(3)).await;
        let latest_block = node_client.get_latest_block().await?.unwrap();

        let collected_transfers = transfer_store
            .get_transfers_between_block_numbers(start_block.number(), latest_block.number())
            .await?;

        assert_eq!(collected_transfers.len(), 3);
        for transfer in &collected_transfers {
            println!("Collected: {transfer:?}");
            let log_tx_hash: TxHash = transfer.transaction_hash.as_slice().try_into().unwrap();
            assert!(expected_tx_hashes.contains(&log_tx_hash));
            expected_tx_hashes.remove(&log_tx_hash);
        }
        assert!(expected_tx_hashes.is_empty());

        // Stop the engine
        engine.shutdown().await;

        // Check balances
        let alice_after = contract.balanceOf(alice).call().await?;
        let bob_after = contract.balanceOf(bob).call().await?;

        assert_eq!(alice_before - alice_after, amount_1 + amount_2);
        assert_eq!(bob_after - bob_before, amount_1 + amount_2);

        println!("✅ Transfer event and balances verified");

        // Re-start the engine
        let latest_checkpoint = checkpoint_store.get_last_checkpoint().await?.unwrap();
        let args = engine::args::Args {
            addresses: ValueOrArray::Value(*contract.address()),
            event: "Transfer(address,address,uint256)".to_string(),
            from_block: BlockHash::from_slice(&latest_checkpoint.block_hash),
            backfill_chunk_size: 2,
            checkpoint_interval: 1,
            poll_interval: Duration::from_millis(100),
        };
        let restarted_engine = Engine::start(
            &args,
            &node_client,
            Arc::clone(&checkpoint_store),
            Arc::clone(&transfer_processor),
        )
        .await?;

        // Let the engine run for a few iterations
        tokio::time::sleep(Duration::from_secs(3)).await;

        // Stop the engine
        restarted_engine.shutdown().await;

        // Check collected results
        let collected_transfers_after_restart =
            transfer_store.get_transfers_from_block_number(start_block.number()).await?;

        assert_eq!(collected_transfers_after_restart.len(), 3);
        for transfer in &collected_transfers_after_restart {
            println!("Collected: {transfer:?}");
        }

        assert_eq!(collected_transfers, collected_transfers_after_restart);

        Ok(())
    }
}
