use alloy::{
    hex,
    primitives::{Address, U256},
};
use std::fmt::{Debug, Formatter, Result};

use crate::model::{Checkpoint, Transfer};

fn bytes_to_hex(bytes: &[u8]) -> String {
    format!("0x{}", hex::encode(bytes))
}

fn bytes_to_address(bytes: &[u8]) -> String {
    match bytes.len() {
        // contract addresses
        20 => match Address::try_from(bytes) {
            Ok(addr) => format!("{addr:?}"),
            Err(_) => format!("<invalid address: {}>", bytes_to_hex(bytes)),
        },
        // account addresses
        32 => match Address::try_from(&bytes[12..]) {
            Ok(addr) => format!("{addr:?}"),
            Err(_) => format!("<invalid address: {}>", bytes_to_hex(bytes)),
        },
        _ => format!("<invalid address length {}: {}>", bytes.len(), bytes_to_hex(bytes)),
    }
}

fn bytes_to_u256(bytes: &[u8]) -> String {
    if bytes.len() <= 32 {
        let mut arr = [0u8; 32];
        arr[32 - bytes.len()..].copy_from_slice(bytes);
        let amount = U256::from_be_bytes(arr);
        format!("{amount}")
    } else {
        format!("<invalid u256: {}>", bytes_to_hex(bytes))
    }
}

impl Debug for Transfer {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.debug_struct("Transfer")
            .field("block_number", &self.block_number)
            .field("block_hash", &bytes_to_hex(&self.block_hash))
            .field("transaction_hash", &bytes_to_hex(&self.transaction_hash))
            .field("log_index", &self.log_index)
            .field("contract_address", &bytes_to_address(&self.contract_address[..]))
            .field("from_address", &bytes_to_address(&self.from_address[..]))
            .field("to_address", &bytes_to_address(&self.to_address[..]))
            .field("amount", &bytes_to_u256(&self.amount))
            .finish()
    }
}

impl Debug for Checkpoint {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.debug_struct("Checkpoint")
            .field("block_number", &self.block_number)
            .field("block_hash", &bytes_to_hex(&self.block_hash))
            .field("parent_hash", &bytes_to_hex(&self.parent_hash))
            .finish()
    }
}
