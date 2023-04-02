use clap::{Parser, Subcommand};
use env_logger::init as log_init;
use log::debug;

pub mod handler;
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    log_init();
    let args = Arguments::parse();
    handler::process(&args).await
}

/// Simple program to start or stop a deployment
#[derive(Parser, Debug)]
pub struct Arguments {
    #[clap(short, long)]
    label: String,
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

impl Arguments {
    pub fn is_stop_action(&self) -> bool {
        match &self.cmd {
            Command::Start { namespace: _ } => {
                debug!("start action selected");
                return false;
            }
            Command::Stop { namespace: _ } => {
                debug!("stop action selected");
                return true;
            }
        }
    }

    pub fn get_namespace(&self) -> &String {
        match &self.cmd {
            Command::Start { namespace } => {
                return &namespace;
            }
            Command::Stop { namespace } => {
                return &namespace;
            }
        }
    }
}
