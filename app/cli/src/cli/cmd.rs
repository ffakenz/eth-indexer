use clap::command;
use clap::{Parser, Subcommand};

use crate::cli::query::args::Query;

use super::engine::args::Args;

#[derive(Parser, Debug)]
#[command(name = "eth-indexer")]
#[command(about = "CLI tool for ETH indexers", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Start eth indexer engine
    Engine(Args),
    Select(Query),
}
