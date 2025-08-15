use alloy::primitives::BlockNumber;
use store::transfer::model::Transfer;

pub trait Outcome {
    fn block_number(&self) -> BlockNumber;
}

impl Outcome for Transfer {
    fn block_number(&self) -> BlockNumber {
        self.block_number as u64
    }
}
