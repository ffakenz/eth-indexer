use std::str::FromStr;

use clap::ValueEnum;

#[derive(Debug, Clone, ValueEnum)]
pub enum Entity {
    Transfer,
    Checkpoint,
}

#[derive(Debug, Clone)]
pub enum FromBlock {
    Number(u64),
    Last,
}

impl FromStr for FromBlock {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.eq_ignore_ascii_case("last") {
            Ok(FromBlock::Last)
        } else {
            s.parse::<u64>()
                .map(FromBlock::Number)
                .map_err(|_| format!("`{s}` is not a valid block number or `last`"))
        }
    }
}
