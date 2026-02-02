use std::time::Instant;

use anyhow::Result;
use serde::Serialize;
use tokio_util::sync::CancellationToken;

use crate::cli::Args;
use crate::delete::delete_directory;
use crate::risk::analyze_risk;
use crate::scanner::{calculate_size, start_scan};

#[derive(Serialize)]
struct JsonOutput {
    version: u8,
    results: Vec<JsonResult>,
    meta: JsonMeta,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct JsonResult {
    path: String,
    size: u64,
    modification_time: Option<u64>,
    risk_analysis: RiskJson,
    deleted: Option<bool>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RiskJson {
    is_sensitive: bool,
    reason: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct JsonMeta {
    results_count: usize,
    run_duration: u64,
}

pub async fn run(args: &Args, cancel_token: CancellationToken) -> Result<()> {
    let start = Instant::now();
    let mut results = Vec::new();

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

    // Collect all results
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
                .map(|d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX));

            let deleted = if args.delete_all {
                let del_result = delete_directory(&result.path, args.dry_run).await;
                Some(del_result.success)
            } else {
                None
            };

            results.push(JsonResult {
                path: result.path.to_string_lossy().to_string(),
                size,
                modification_time,
                risk_analysis: RiskJson {
                    is_sensitive: risk.is_sensitive,
                    reason: risk.reason,
                },
                deleted,
            });
        }
    }

    let results_count = results.len();
    let output = JsonOutput {
        version: 1,
        results,
        meta: JsonMeta {
            results_count,
            run_duration: u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX),
        },
    };

    println!("{}", serde_json::to_string_pretty(&output)?);

    Ok(())
}
