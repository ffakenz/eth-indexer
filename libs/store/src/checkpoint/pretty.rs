use std::fmt::{Debug, Formatter, Result};

use crate::{checkpoint::model::Checkpoint, utils};

impl Debug for Checkpoint {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.debug_struct("Checkpoint")
            .field("block_number", &self.block_number)
            .field("block_hash", &utils::bytes_to_hex(&self.block_hash))
            .field("parent_hash", &utils::bytes_to_hex(&self.parent_hash))
            .finish()
    }
}
