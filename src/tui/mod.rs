mod analytics;
mod app;
mod cleanup;
mod event_loop;
mod input;
mod panels;
mod ui;
mod widgets;

use anyhow::Result;
use tokio_util::sync::CancellationToken;

use crate::cli::Args;

pub async fn run(args: &Args, cancel_token: CancellationToken) -> Result<()> {
    event_loop::run(args, cancel_token).await
}
