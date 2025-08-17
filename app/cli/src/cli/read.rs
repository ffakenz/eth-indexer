use alloy::{primitives::Address, rpc::types::ValueOrArray};
use clap::ValueEnum;
use engine::live::source::filter::EventType;

#[derive(Debug, Clone, ValueEnum)]
pub enum CliEventType {
    Transfer,
}

impl From<CliEventType> for EventType {
    fn from(value: CliEventType) -> Self {
        match value {
            CliEventType::Transfer => EventType::Transfer,
        }
    }
}

pub fn parse_addresses(input: &str) -> ValueOrArray<Address> {
    let parts: Vec<_> =
        input.split(',').map(|s| s.trim().parse::<Address>().expect("Invalid address")).collect();

    if parts.len() == 1 { ValueOrArray::Value(parts[0]) } else { ValueOrArray::Array(parts) }
}
