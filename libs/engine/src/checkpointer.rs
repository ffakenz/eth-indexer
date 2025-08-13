use alloy::hex;
use alloy::rpc::types::Block;
use eyre::{Result, eyre};
use sqlx::Error;
use store::checkpoint::model::Checkpoint;
use store::checkpoint::store::Store as CheckpointStore;

pub async fn save_checkpoint(
    checkpoint_block: &Block,
    checkpoint_store: &CheckpointStore,
) -> Result<()> {
    let block_number = checkpoint_block.number();
    let block_hash = checkpoint_block.hash();
    let checkpoint = Checkpoint {
        block_number: block_number as i64,
        block_hash: block_hash.to_vec(),
        parent_hash: checkpoint_block.header.parent_hash.to_vec(),
    };

    match checkpoint_store.insert_checkpoint(&checkpoint).await {
        Ok(_) => {
            println!("Checkpoint saved at block number {block_number:?} and hash {block_hash:?}");
            Ok(())
        }
        Err(e) => {
            if let Error::Database(db_err) = &e {
                if db_err.message().contains(
                    "UNIQUE constraint failed: checkpoints.block_number, checkpoints.block_hash",
                ) {
                    println!(
                        "Duplicate checkpoint ignored: block_number={}, block_hash={}",
                        hex::encode(&checkpoint.block_hash),
                        checkpoint.block_number
                    );
                    Ok(())
                } else {
                    eprintln!("Checkpointer failed on [insert_checkpoint]: {e:?}");
                    Err(eyre!(e))
                }
            } else {
                eprintln!("Checkpointer fatal error on [insert_checkpoint]: {e:?}");
                Err(eyre!(e))
            }
        }
    }
}
