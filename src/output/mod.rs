mod json;
mod stream;

use anyhow::Result;
use tokio_util::sync::CancellationToken;

use crate::cli::Args;

pub async fn run_non_interactive(args: &Args, cancel_token: CancellationToken) -> Result<()> {
    if args.json_stream {
        stream::run(args, cancel_token).await
    } else {
        json::run(args, cancel_token).await
    }
}
