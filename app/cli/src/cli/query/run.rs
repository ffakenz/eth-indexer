use crate::cli::query::args::Query;
use crate::cli::query::read::{Entity, FromBlock};
use crate::cli::query::response::{CheckpointResponse, TransferResponse};
use eyre::{Result, eyre};
use store::client::Client;

pub async fn select(query: &Query) -> Result<()> {
    let client = Client::init(&query.db_url).await?;

    let checkpoint_store = store::checkpoint::store::Store::new(client.clone());

    let from_block_number = match query.from_block {
        FromBlock::Number(block_number) => Ok(block_number),
        FromBlock::Last => match checkpoint_store.get_last_checkpoint().await? {
            None => Err(eyre!("Last Checkpoint Not Found")),
            Some(checkpoint) => Ok(checkpoint.block_number as u64),
        },
    };

    match query.entity {
        Entity::Transfer => {
            let transfer_store = store::transfer::store::Store::new(client.clone());

            let block_number = from_block_number?;

            let transfers = transfer_store.get_transfers_from_block_number(block_number).await?;

            if transfers.is_empty() {
                println!("No Transfers Found")
            } else {
                let response: Vec<TransferResponse> =
                    transfers.into_iter().map(TransferResponse).collect();
                println!("{}", serde_json::to_string_pretty(&response).unwrap());
            }
        }
        Entity::Checkpoint => {
            let block_number = from_block_number?;

            let checkpoint = checkpoint_store
                .get_checkpoint_by_number(block_number)
                .await?
                .ok_or(eyre!("Checkpoint Not Found"))?;

            let response = CheckpointResponse(checkpoint);

            println!("{}", serde_json::to_string_pretty(&response).unwrap());
        }
    }

    Ok(())
}
