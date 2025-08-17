use std::time::Duration;

use alloy::eips::BlockId;
use alloy::eips::BlockNumberOrTag;
use alloy::network::EthereumWallet;
use alloy::primitives::Address;
use alloy::primitives::BlockHash;
use alloy::providers::Identity;
use alloy::providers::RootProvider;
use alloy::providers::fillers::{
    BlobGasFiller, ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller, WalletFiller,
};
use alloy::providers::{Provider, ProviderBuilder};
use alloy::rpc::client::PollerStream;
use alloy::rpc::client::RpcClient;
use alloy::rpc::types::Block;
use alloy::rpc::types::Filter;
use alloy::rpc::types::Log;
use alloy::rpc::types::ValueOrArray;
use alloy::signers::local::PrivateKeySigner;
use alloy::transports::RpcError;
use alloy::transports::TransportErrorKind;
use alloy::transports::http::reqwest;
use eyre::Result;
use futures_util::StreamExt;
use futures_util::stream::BoxStream;
use reqwest::Url;

type NodeClientProvider = FillProvider<
    JoinFill<
        JoinFill<
            Identity,
            JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>,
        >,
        WalletFiller<EthereumWallet>,
    >,
    RootProvider,
>;

#[derive(Clone)]
pub struct NodeClient {
    provider: NodeClientProvider,
}

impl NodeClient {
    pub fn new(rpc_url: Url, signer: PrivateKeySigner) -> Self {
        let rpc_client = RpcClient::new_http(rpc_url);
        let provider = ProviderBuilder::new().wallet(signer).connect_client(rpc_client);
        Self { provider }
    }

    pub fn borrow_provider(&self) -> &NodeClientProvider {
        &self.provider
    }

    pub async fn get_latest_block_number(&self) -> Result<u64, RpcError<TransportErrorKind>> {
        self.provider.get_block_number().await
    }

    pub async fn get_latest_block(&self) -> Result<Option<Block>, RpcError<TransportErrorKind>> {
        self.provider.get_block(BlockId::latest()).full().await
    }

    pub async fn get_latest_finalized_block(
        &self,
    ) -> Result<Option<Block>, RpcError<TransportErrorKind>> {
        self.provider.get_block(BlockId::finalized()).full().await
    }

    pub async fn get_accounts(&self) -> Result<Vec<Address>, RpcError<TransportErrorKind>> {
        self.provider.get_accounts().await
    }

    pub async fn get_logs(
        &self,
        addresses: ValueOrArray<Address>,
        event: &str,
        from_block_number: BlockNumberOrTag,
        to_block_number: BlockNumberOrTag,
    ) -> Result<Vec<Log>, RpcError<TransportErrorKind>> {
        let filter = Filter::new()
            .address(addresses)
            .event(event)
            .from_block(from_block_number)
            .to_block(to_block_number);

        self.provider.get_logs(&filter).await
    }

    pub async fn watch_logs(
        &self,
        addresses: ValueOrArray<Address>,
        event: &str,
        from_block_number: BlockNumberOrTag,
        poll_interval: Duration,
    ) -> Result<PollerStream<Vec<Log>>, RpcError<TransportErrorKind>> {
        let filter = Filter::new().address(addresses).event(event).from_block(from_block_number);

        self.provider
            .watch_logs(&filter)
            .await
            .map(|poller_builder| poller_builder.with_poll_interval(poll_interval).into_stream())
    }

    pub async fn watch_block_hashes(
        &self,
        poll_interval: Duration,
    ) -> Result<PollerStream<Vec<BlockHash>>, RpcError<TransportErrorKind>> {
        self.provider
            .watch_blocks()
            .await
            .map(|block_provider| block_provider.with_poll_interval(poll_interval).into_stream())
    }

    pub async fn watch_full_blocks<'a>(
        &'a self,
        poll_interval: Duration,
    ) -> Result<BoxStream<'a, Result<Block, RpcError<TransportErrorKind>>>> {
        let mut watcher = self.provider.watch_full_blocks().await?;
        watcher.set_poll_interval(poll_interval);
        Ok(watcher.into_stream().boxed())
    }

    pub async fn get_block_by_hash(
        &self,
        block_hash: BlockHash,
    ) -> Result<Option<Block>, RpcError<TransportErrorKind>> {
        self.provider.get_block_by_hash(block_hash).await
    }

    pub async fn get_block_by_number(
        &self,
        block_number: u64,
    ) -> Result<Option<Block>, RpcError<TransportErrorKind>> {
        self.provider.get_block_by_number(block_number.into()).await
    }

    pub async fn get_block_by_id(
        &self,
        block_id: BlockId,
    ) -> Result<Option<Block>, RpcError<TransportErrorKind>> {
        self.provider.get_block(block_id).await
    }
}
