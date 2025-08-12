use crate::model::{Checkpoint, Transfer};
use alloy::{eips::BlockNumHash, rpc::types::Log};
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
            data: log.data().data.to_vec(),
        })
    }
}

impl From<&BlockNumHash> for Checkpoint {
    fn from(block_num_hash: &BlockNumHash) -> Self {
        Self {
            block_number: block_num_hash.number as i64,
            block_hash: block_num_hash.hash.to_vec(),
        }
    }
}
