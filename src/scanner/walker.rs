use std::path::PathBuf;
use std::sync::Arc;
use std::time::SystemTime;

use ignore::{WalkBuilder, WalkState};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use super::batcher::ResultBatcher;

#[derive(Debug, Clone)]
pub struct ScanResult {
    pub path: PathBuf,
    pub size: Option<u64>,
    pub file_count: Option<u64>,
    pub modified: Option<SystemTime>,
    pub is_sensitive: bool,
}

impl ScanResult {
    pub fn new(path: PathBuf) -> Self {
        let modified = std::fs::metadata(&path)
            .ok()
            .and_then(|m| m.modified().ok());

        Self {
            path,
            size: None,
            file_count: None,
            modified,
            is_sensitive: false,
        }
    }
}

pub fn start_scan(
    root: PathBuf,
    targets: Vec<String>,
    excludes: Vec<String>,
    follow_links: bool,
    respect_ignore: bool,
    cancel_token: CancellationToken,
) -> mpsc::UnboundedReceiver<Vec<ScanResult>> {
    let (tx, rx) = mpsc::unbounded_channel();

    tokio::task::spawn_blocking(move || {
        let targets = Arc::new(targets);
        let excludes = Arc::new(excludes);
        let batcher = Arc::new(std::sync::Mutex::new(ResultBatcher::new(tx.clone())));

        WalkBuilder::new(&root)
            .hidden(false) // Scan hidden dirs (.pnpm-store, .yarn)
            .follow_links(follow_links) // SAFETY: default false
            .git_ignore(respect_ignore) // SAFETY: default false - scan everything
            .git_global(false)
            .git_exclude(false)
            .threads(num_cpus::get())
            .build_parallel()
            .run(|| {
                let targets = Arc::clone(&targets);
                let excludes = Arc::clone(&excludes);
                let batcher = Arc::clone(&batcher);
                let cancel = cancel_token.clone();

                Box::new(move |result| {
                    if cancel.is_cancelled() {
                        return WalkState::Quit;
                    }

                    if let Ok(entry) = result {
                        if !entry.file_type().is_some_and(|ft| ft.is_dir()) {
                            return WalkState::Continue;
                        }

                        let file_name = entry.file_name().to_string_lossy();

                        // Check if excluded
                        if excludes.iter().any(|e| file_name == *e) {
                            return WalkState::Skip;
                        }

                        // Check if target
                        if targets.iter().any(|t| file_name == *t) {
                            let result = ScanResult::new(entry.path().to_path_buf());
                            batcher
                                .lock()
                                .unwrap_or_else(std::sync::PoisonError::into_inner)
                                .add(result);
                            // Don't descend into matched directory
                            return WalkState::Skip;
                        }
                    }

                    WalkState::Continue
                })
            });

        // Flush remaining results
        batcher
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .flush();
    });

    rx
}
