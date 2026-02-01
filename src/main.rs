mod cli;
mod delete;
mod output;
mod profiles;
mod risk;
mod scanner;
mod tui;

use anyhow::Result;
use clap::Parser;
use cli::Args;
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let cancel_token = CancellationToken::new();

    // Handle Ctrl+C gracefully
    let cancel_clone = cancel_token.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        cancel_clone.cancel();
    });

    // Determine output mode
    if args.json || args.json_stream {
        output::run_non_interactive(&args, cancel_token).await
    } else {
        tui::run(&args, cancel_token).await
    }
}
