use alloy::{
    hex,
    primitives::{Address, U256},
};

pub fn bytes_to_hex(bytes: &[u8]) -> String {
    format!("0x{}", hex::encode(bytes))
}

pub fn bytes_to_address(bytes: &[u8]) -> String {
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

pub fn bytes_to_u256(bytes: &[u8]) -> String {
    if bytes.len() <= 32 {
        let mut arr = [0u8; 32];
        arr[32 - bytes.len()..].copy_from_slice(bytes);
        let amount = U256::from_be_bytes(arr);
        format!("{amount}")
    } else {
        format!("<invalid u256: {}>", bytes_to_hex(bytes))
    }
}
