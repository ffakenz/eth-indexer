use crate::checkpoint::model::Checkpoint;
use alloy::rpc::types::Block;

impl From<&Block> for Checkpoint {
    fn from(block: &Block) -> Self {
        let block_number = block.number();
        let block_hash = block.hash();
        Checkpoint {
            block_number: block_number as i64,
            block_hash: block_hash.to_vec(),
            parent_hash: block.header.parent_hash.to_vec(),
        }
    }
}
