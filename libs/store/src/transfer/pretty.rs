use std::fmt::{Debug, Formatter, Result};

use crate::{transfer::model::Transfer, utils};

impl Debug for Transfer {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.debug_struct("Transfer")
            .field("block_number", &self.block_number)
            .field("block_hash", &utils::bytes_to_hex(&self.block_hash))
            .field("transaction_hash", &utils::bytes_to_hex(&self.transaction_hash))
            .field("log_index", &self.log_index)
            .field("contract_address", &utils::bytes_to_address(&self.contract_address[..]))
            .field("from_address", &utils::bytes_to_address(&self.from_address[..]))
            .field("to_address", &utils::bytes_to_address(&self.to_address[..]))
            .field("amount", &utils::bytes_to_u256(&self.amount))
            .finish()
    }
}
