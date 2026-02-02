use std::path::Path;

use thiserror::Error;

#[derive(Debug, Clone)]
pub struct DeleteResult {
    pub success: bool,
    #[allow(dead_code)]
    pub size_freed: u64,
    pub error: Option<String>,
}

#[allow(dead_code)]
#[derive(Error, Debug)]
pub enum DeleteError {
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    #[error("Path not found: {0}")]
    NotFound(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub async fn delete_directory(path: &Path, dry_run: bool) -> DeleteResult {
    let path = path.to_path_buf();

    if dry_run {
        // Simulate deletion with a short delay
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        return DeleteResult {
            success: true,
            size_freed: 0,
            error: None,
        };
    }

    tokio::task::spawn_blocking(move || {
        match std::fs::remove_dir_all(&path) {
            Ok(()) => DeleteResult {
                success: true,
                size_freed: 0, // Size should be calculated before deletion
                error: None,
            },
            Err(e) => DeleteResult {
                success: false,
                size_freed: 0,
                error: Some(e.to_string()),
            },
        }
    })
    .await
    .unwrap_or_else(|e| DeleteResult {
        success: false,
        size_freed: 0,
        error: Some(e.to_string()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_dry_run_does_not_delete() {
        let temp = tempdir().unwrap();
        let target = temp.path().join("to_delete");
        fs::create_dir(&target).unwrap();
        fs::write(target.join("file.txt"), "content").unwrap();

        let result = delete_directory(&target, true).await;

        assert!(result.success);
        assert!(result.error.is_none());
        assert!(target.exists(), "dry_run should not delete");
    }

    #[tokio::test]
    async fn test_actual_deletion() {
        let temp = tempdir().unwrap();
        let target = temp.path().join("to_delete");
        fs::create_dir(&target).unwrap();
        fs::write(target.join("file.txt"), "content").unwrap();

        let result = delete_directory(&target, false).await;

        assert!(result.success);
        assert!(result.error.is_none());
        assert!(!target.exists(), "should delete directory");
    }

    #[tokio::test]
    async fn test_delete_nonexistent_returns_error() {
        let result = delete_directory(Path::new("/nonexistent/path/xyz123"), false).await;

        assert!(!result.success);
        assert!(result.error.is_some());
    }

    #[tokio::test]
    async fn test_delete_nested_directory() {
        let temp = tempdir().unwrap();
        let target = temp.path().join("outer");
        let inner = target.join("inner").join("deep");
        fs::create_dir_all(&inner).unwrap();
        fs::write(inner.join("file.txt"), "nested").unwrap();

        let result = delete_directory(&target, false).await;

        assert!(result.success);
        assert!(!target.exists());
    }
}
