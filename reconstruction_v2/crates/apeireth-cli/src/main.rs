use clap::Parser;
use apeireth_cli::{Cli, run_cli};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let cli = Cli::parse();
    run_cli(cli).await
}

