mod cli {
    pub mod engine {
        pub mod args;
        pub mod run;
    }
    pub mod query {
        pub mod args;
        pub mod read;
        pub mod response;
        pub mod run;
    }
    pub mod cmd;
    pub mod read;
}

use clap::Parser;
use engine::args::Args;
use eyre::Result;
use std::time::Duration;

use crate::cli::cmd::{Cli, Command};
use crate::cli::read;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    init_tracing(&cli);

    match &cli.command {
        Command::Engine(args) => {
            tracing::info!("Engine Command: {:?}", args);
            // map CLI args to Engine Args
            let start_args = Args {
                addresses: read::parse_addresses(&args.addresses),
                event: args.event.clone().into(),
                from_block: args.from_block,
                poll_interval: Duration::from_millis(args.poll_interval),
                checkpoint_interval: args.checkpoint_interval,
                backfill_checkpoint_interval: args.backfill_checkpoint_interval,
            };
            cli::engine::run::start(&args.rpc_url, &args.db_url, &args.signer_pk, start_args).await
        }
        Command::Select(query) => {
            tracing::info!("Engine Query: {:?}", query);
            cli::query::run::select(query).await
        }
    }
}

fn init_tracing(cli: &Cli) {
    match &cli.command {
        Command::Engine(_) => {
            // install global subscriber configured based on RUST_LOG envvar.
            tracing_subscriber::fmt::init();
        }
        Command::Select(_) => {
            tracing_subscriber::fmt::Subscriber::builder().with_writer(std::io::stderr).init();
        }
    }
}
