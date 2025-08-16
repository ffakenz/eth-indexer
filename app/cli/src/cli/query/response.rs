use serde::{Serialize, ser::SerializeStruct};
use store::{checkpoint::model::Checkpoint, transfer::model::Transfer, utils};

// Tuple wrapper for Checkpoint
pub struct CheckpointResponse(pub Checkpoint);

impl Serialize for CheckpointResponse {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let c = &self.0;
        let mut state = serializer.serialize_struct("Checkpoint", 3)?;
        state.serialize_field("block_number", &c.block_number)?;
        state.serialize_field("block_hash", &utils::bytes_to_hex(&c.block_hash))?;
        state.serialize_field("parent_hash", &utils::bytes_to_hex(&c.parent_hash))?;
        state.end()
    }
}

// Tuple wrapper for Transfer
pub struct TransferResponse(pub Transfer);

impl Serialize for TransferResponse {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let t = &self.0;
        let mut state = serializer.serialize_struct("Transfer", 8)?;
        state.serialize_field("block_number", &t.block_number)?;
        state.serialize_field("block_hash", &utils::bytes_to_hex(&t.block_hash))?;
        state.serialize_field("transaction_hash", &utils::bytes_to_hex(&t.transaction_hash))?;
        state.serialize_field("log_index", &t.log_index)?;
        state.serialize_field("contract_address", &utils::bytes_to_address(&t.contract_address))?;
        state.serialize_field("from_address", &utils::bytes_to_address(&t.from_address))?;
        state.serialize_field("to_address", &utils::bytes_to_address(&t.to_address))?;
        state.serialize_field("amount", &utils::bytes_to_u256(&t.amount))?;
        state.end()
    }
}
