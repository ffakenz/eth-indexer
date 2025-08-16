use clap::Parser;
use clap::{arg, command};

use crate::cli::read::CliEventType;

#[derive(Parser, Debug)]
#[command(about = "Start the ETH indexer", long_about = None)]
pub struct Args {
    /// Node Provider connection string
    #[arg(short, long)]
    pub rpc_url: String,

    /// SQLite connection string
    #[arg(short, long)]
    pub db_url: String,

    /// User signing private key
    #[arg(short, long)]
    pub signer_pk: String,

    /// Addresses to watch (comma-separated)
    #[arg(short, long)]
    pub addresses: String,

    /// Event type to watch
    #[arg(short, long, value_enum)]
    pub event: CliEventType,

    /// From block number to watch
    #[arg(long, default_value_t = 0)]
    pub from_block: u64,

    /// Checkpoint interval
    #[arg(long, default_value_t = 100)]
    pub checkpoint_interval: u64,

    /// Poll interval in milliseconds
    #[arg(long, default_value_t = 500)]
    pub poll_interval: u64,
}
