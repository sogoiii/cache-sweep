use std::path::Path;

use tokio::sync::Semaphore;

static SIZE_SEMAPHORE: Semaphore = Semaphore::const_new(10);

/// Returns (total_size, file_count)
pub async fn calculate_size(path: &Path) -> (u64, u64) {
    let _permit = SIZE_SEMAPHORE.acquire().await.ok();
    let path = path.to_path_buf();

    tokio::task::spawn_blocking(move || calculate_dir_size(&path))
        .await
        .unwrap_or((0, 0))
}

/// Returns (total_size, file_count)
fn calculate_dir_size(path: &Path) -> (u64, u64) {
    let mut total_size = 0u64;
    let mut file_count = 0u64;

    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let metadata = match entry.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };

            if metadata.is_file() {
                total_size += metadata.len();
                file_count += 1;
            } else if metadata.is_dir() {
                let (sub_size, sub_count) = calculate_dir_size(&entry.path());
                total_size += sub_size;
                file_count += sub_count;
            }
        }
    }

    (total_size, file_count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_calculate_dir_size_empty() {
        let dir = tempdir().unwrap();
        let (size, count) = calculate_dir_size(dir.path());
        assert_eq!(size, 0);
        assert_eq!(count, 0);
    }

    #[test]
    fn test_calculate_dir_size_single_file() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        let mut file = File::create(&file_path).unwrap();
        file.write_all(b"hello").unwrap();

        let (size, count) = calculate_dir_size(dir.path());
        assert_eq!(size, 5);
        assert_eq!(count, 1);
    }

    #[test]
    fn test_calculate_dir_size_multiple_files() {
        let dir = tempdir().unwrap();

        let mut f1 = File::create(dir.path().join("a.txt")).unwrap();
        f1.write_all(b"aaa").unwrap();

        let mut f2 = File::create(dir.path().join("b.txt")).unwrap();
        f2.write_all(b"bbbbb").unwrap();

        let (size, count) = calculate_dir_size(dir.path());
        assert_eq!(size, 8); // 3 + 5
        assert_eq!(count, 2);
    }

    #[test]
    fn test_calculate_dir_size_nested() {
        let dir = tempdir().unwrap();

        // Create nested structure
        let sub = dir.path().join("subdir");
        fs::create_dir(&sub).unwrap();

        let mut f1 = File::create(dir.path().join("root.txt")).unwrap();
        f1.write_all(b"root").unwrap();

        let mut f2 = File::create(sub.join("nested.txt")).unwrap();
        f2.write_all(b"nested").unwrap();

        let (size, count) = calculate_dir_size(dir.path());
        assert_eq!(size, 10); // 4 + 6
        assert_eq!(count, 2);
    }

    #[test]
    fn test_calculate_dir_size_nonexistent() {
        let (size, count) = calculate_dir_size(Path::new("/nonexistent/path/12345"));
        assert_eq!(size, 0);
        assert_eq!(count, 0);
    }
}
