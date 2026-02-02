use anyhow::Result;
use serde::Serialize;
use tokio_util::sync::CancellationToken;

use crate::cli::Args;
use crate::delete::delete_directory;
use crate::risk::analyze_risk;
use crate::scanner::{calculate_size, start_scan};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StreamResult {
    path: String,
    size: u64,
    modification_time: Option<u64>,
    is_sensitive: bool,
    deleted: Option<bool>,
}

pub async fn run(args: &Args, cancel_token: CancellationToken) -> Result<()> {
    let root = args.effective_directory();
    let targets = args.effective_targets();
    let excludes = args.exclude.clone().unwrap_or_default();

    let mut rx = start_scan(
        root,
        targets,
        excludes,
        args.follow_links,
        args.respect_ignore,
        cancel_token.clone(),
    );

    while let Some(batch) = rx.recv().await {
        for result in batch {
            if cancel_token.is_cancelled() {
                break;
            }

            let risk = analyze_risk(&result.path);

            if args.exclude_sensitive && risk.is_sensitive {
                continue;
            }

            let (size, _file_count) = calculate_size(&result.path).await;
            let modification_time = result
                .modified
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_millis() as u64);

            let mut deleted = None;
            if args.delete_all {
                let del_result = delete_directory(&result.path, args.dry_run).await;
                deleted = Some(del_result.success);
            }

            let stream_result = StreamResult {
                path: result.path.to_string_lossy().to_string(),
                size,
                modification_time,
                is_sensitive: risk.is_sensitive,
                deleted,
            };

            // One JSON object per line
            println!("{}", serde_json::to_string(&stream_result)?);
        }
    }

    Ok(())
}
