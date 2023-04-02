use clap::{Parser, Subcommand};

pub mod handler;
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Arguments::parse();
    handler::process(&args).await
}

/// Simple program to start or stop a deployment
#[derive(Parser, Debug)]
pub struct Arguments {
    #[clap(subcommand)]
    cmd: Command,
}

/// Simple first progra
#[derive(Subcommand, Debug)]
#[command(author="btc", version="0.0.0", about="about my first rust cli", long_about = None)]
enum Command {
    /// Start a deployment
    Start {
        #[arg(short, long, required = true)]
        namespace: String,
    },
    /// list all the projects
    Stop {
        #[arg(short, long, required = true)]
        namespace: String,
    },
}
