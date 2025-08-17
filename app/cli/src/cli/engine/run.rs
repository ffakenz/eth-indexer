use alloy::{rpc::types::Log, signers::local::PrivateKeySigner, transports::http::reqwest::Url};
use chain::rpc::NodeClient;
use engine::{
    args::Args,
    engine::Engine,
    live::sink::{handle::Sink, transfer::TransferSink},
    live::source::{handle::Source, log::LogSource},
};
use eyre::Result;
use std::{str::FromStr, sync::Arc};
use store::{client::Client, transfer::model::Transfer};

pub async fn start(rpc_url: &str, db_url: &str, signer_pk: &str, engine_args: Args) -> Result<()> {
    // Start engine
    let node_client = NodeClient::new(Url::parse(rpc_url)?, PrivateKeySigner::from_str(signer_pk)?);
    let source: Arc<dyn Source<Item = Log>> =
        Arc::new(LogSource { node_client: node_client.clone() });

    let client = Client::init(db_url).await?;
    let checkpoint_store = Arc::new(store::checkpoint::store::Store::new(client.clone()));
    let transfer_store = store::transfer::store::Store::new(client.clone());
    let sink: Arc<dyn Sink<Item = Transfer>> = Arc::new(TransferSink { store: transfer_store });

    tracing::info!("Starting the engine {engine_args:?}");

    let engine = Engine::start(&engine_args, &node_client, source, checkpoint_store, sink).await?;

    // Wait for user to request shutdown (SIGINT)
    tokio::signal::ctrl_c().await?;
    tracing::info!("Shutting down engine...");

    // Gracefully shutdown
    engine.shutdown().await;

    Ok(())
}
