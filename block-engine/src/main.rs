use clap::Parser;
use tracing::info;

mod bundle;
mod auction;
mod transaction_pool;
mod simulator;
mod block_assembler;

#[cfg(test)]
mod integration_tests;

use crate::bundle::BundleEngine;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[arg(short, long, default_value = "127.0.0.1:8080")]
    bind_address: String,
    
    #[arg(short, long, default_value = "https://api.mainnet-beta.solana.com")]
    rpc_url: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    
    let cli = Cli::parse();
    info!("Starting Permissionless Block Engine on {}", cli.bind_address);
    
    let mut engine = BundleEngine::new(cli.rpc_url).await?;
    engine.start_auction_loop().await?;
    
    Ok(())
}
