#[cfg(test)]
mod tests {
    use engine::engine::Engine;
    use eyre::Result;

    use std::{collections::HashSet, time::Duration};

    use alloy::{
        eips::BlockNumberOrTag,
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
        "../../tests/resources/zamatoken/ZamaToken.json"
    );

    #[tokio::test]
    async fn test_polling_transfer_event_logs() -> Result<()> {
        // Spin up a local Anvil node.
        // Ensure `anvil` is available in $PATH.
        let anvil = Anvil::new().block_time(1).try_spawn()?;
        let pk: PrivateKeySigner = anvil.keys()[0].clone().into();
        let owner: Address = pk.address();

        // Create a RPC client provider.
        let rpc_url = anvil.endpoint_url();
        let node_client = NodeClient::new(rpc_url, pk);

        // Deploy the `ZamaToken` contract.
        println!("Deploying contract...");
        println!("Signer: {owner:?}");
        let contract = ZamaToken::deploy(node_client.borrow_provider(), owner).await?;
        println!("Deployed ERC20 contract at: {}", contract.address());
        println!("Owner: {:?}", contract.owner().call().await?);

        // Get two accounts from Anvil, Alice and Bob.
        let accounts = node_client.get_accounts().await?;
        let alice = accounts[0];
        let bob = accounts[1];

        // Fetch latest block after scenario setup
        let latest_block = node_client.get_latest_block_number().await?;
        println!("Latest block: {latest_block:?}");

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
        let tx_hash_1 = contract.transfer(bob, amount_1).send().await?.watch().await?;
        println!("Sent transfer tx: {tx_hash_1}");
        expected_tx_hashes.insert(tx_hash_1);

        // Start the engine
        let args = engine::args::Args {
            address: *contract.address(),
            event: "Transfer(address,address,uint256)".to_string(),
            from_block: BlockNumberOrTag::Number(latest_block),
            poll_interval: Duration::from_millis(100),
        };
        let engine = Engine::start(args, &node_client).await?;

        // Send transfer 2 from Alice -> Bob (after engine startup)
        let amount_2 = U256::from(100);
        let tx_hash_2 = contract.transfer(bob, amount_2).send().await?.watch().await?;
        println!("Sent transfer tx2: {tx_hash_2}");
        expected_tx_hashes.insert(tx_hash_2);

        // Let the engine run for a few iterations
        tokio::time::sleep(Duration::from_secs(1)).await;
        let mut collected_logs = engine.get_collected_logs().await;
        while let Some(log) = collected_logs.pop() {
            println!("Collected Transfer: {log:?}");
            let log_tx_hash = &log.transaction_hash.unwrap();
            assert!(expected_tx_hashes.contains(log_tx_hash));
            expected_tx_hashes.remove(log_tx_hash);
        }

        // Stop the engine
        engine.shutdown().await;

        // Check balances
        let alice_after = contract.balanceOf(alice).call().await?;
        let bob_after = contract.balanceOf(bob).call().await?;

        assert_eq!(alice_before - alice_after, amount_1 + amount_2);
        assert_eq!(bob_after - bob_before, amount_1 + amount_2);

        println!("✅ Transfer event and balances verified");
        Ok(())
    }
}
