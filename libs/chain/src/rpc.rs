use std::time::Duration;

use alloy::eips::BlockNumberOrTag;
use alloy::network::EthereumWallet;
use alloy::primitives::Address;
use alloy::providers::Identity;
use alloy::providers::RootProvider;
use alloy::providers::fillers::{
    BlobGasFiller, ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller, WalletFiller,
};
use alloy::providers::{Provider, ProviderBuilder};
use alloy::rpc::client::PollerStream;
use alloy::rpc::client::RpcClient;
use alloy::rpc::types::Filter;
use alloy::rpc::types::Log;
use alloy::signers::local::PrivateKeySigner;
use alloy::transports::RpcError;
use alloy::transports::TransportErrorKind;
use alloy::transports::http::reqwest;
use eyre::Result;
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

    pub async fn get_accounts(&self) -> Result<Vec<Address>, RpcError<TransportErrorKind>> {
        self.provider.get_accounts().await
    }

    pub async fn watch_logs(
        &self,
        address: &Address,
        event: &str,
        from_block: BlockNumberOrTag,
        poll_interval: Duration,
    ) -> Result<PollerStream<Vec<Log>>, RpcError<TransportErrorKind>> {
        let filter = Filter::new().address(*address).event(event).from_block(from_block);

        self.provider
            .watch_logs(&filter)
            .await
            .map(|poller_builder| poller_builder.with_poll_interval(poll_interval).into_stream())
    }
}
