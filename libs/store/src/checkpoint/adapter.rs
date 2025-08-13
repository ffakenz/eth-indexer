use crate::checkpoint::model::Checkpoint;
use alloy::{eips::BlockNumHash, primitives::BlockHash};

pub fn from(block_num_hash: &BlockNumHash, parent_block_hash: BlockHash) -> Checkpoint {
    Checkpoint {
        block_number: block_num_hash.number as i64,
        block_hash: block_num_hash.hash.to_vec(),
        parent_hash: parent_block_hash.to_vec(),
    }
}
