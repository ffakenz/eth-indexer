use crate::model::{Checkpoint, Transfer};
use alloy::{eips::BlockNumHash, primitives::BlockHash, rpc::types::Log};
use eyre::{Result, eyre};
use std::convert::TryFrom;

impl TryFrom<&Log> for Transfer {
    type Error = eyre::Report;

    fn try_from(log: &Log) -> Result<Self> {
        Ok(Self {
            block_number: log.block_number.ok_or_else(|| eyre!("missing block_number"))? as i64,
            block_hash: log.block_hash.ok_or_else(|| eyre!("missing block_hash"))?.to_vec(),
            transaction_hash: log
                .transaction_hash
                .ok_or_else(|| eyre!("missing transaction_hash"))?
                .to_vec(),
            log_index: log.log_index.ok_or_else(|| eyre!("missing log_index"))? as i64,
            contract_address: log.address().to_vec(),
            from_address: log
                .topics()
                .get(1)
                .ok_or_else(|| eyre!("missing from"))?
                .as_slice()
                .to_vec(),
            to_address: log.topics().get(2).ok_or_else(|| eyre!("missing to"))?.as_slice().to_vec(),
            amount: log.data().data.to_vec(),
        })
    }
}

pub fn from(block_num_hash: &BlockNumHash, parent_block_hash: BlockHash) -> Checkpoint {
    Checkpoint {
        block_number: block_num_hash.number as i64,
        block_hash: block_num_hash.hash.to_vec(),
        parent_hash: parent_block_hash.to_vec(),
    }
}
