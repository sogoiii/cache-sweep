use std::path::Path;

use tokio::sync::Semaphore;

static SIZE_SEMAPHORE: Semaphore = Semaphore::const_new(10);

pub async fn calculate_size(path: &Path) -> u64 {
    let _permit = SIZE_SEMAPHORE.acquire().await.ok();
    let path = path.to_path_buf();

    tokio::task::spawn_blocking(move || calculate_dir_size(&path))
        .await
        .unwrap_or(0)
}

fn calculate_dir_size(path: &Path) -> u64 {
    let mut total = 0u64;

    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let metadata = match entry.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };

            if metadata.is_file() {
                total += metadata.len();
            } else if metadata.is_dir() {
                total += calculate_dir_size(&entry.path());
            }
        }
    }

    total
}
