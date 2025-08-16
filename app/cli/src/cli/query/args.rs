use clap::Parser;

use crate::cli::query::read::{Entity, FromBlock};

#[derive(Parser, Debug)]
#[command(about = "Select indexed results", long_about = None)]
pub struct Query {
    /// SQLite connection string
    #[arg(short, long)]
    pub db_url: String,

    /// Entity to query
    #[arg(short, long, value_enum)]
    pub entity: Entity,

    /// From block number to watch
    #[arg(long)]
    pub from_block: FromBlock,
}
