#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn cache_sweep_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_cache-sweep"))
}

#[test]
fn test_help_flag() {
    let output = Command::new(cache_sweep_bin())
        .arg("--help")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("cache-sweep"));
    assert!(stdout.contains("--profiles"));
    assert!(stdout.contains("--directory"));
}

#[test]
fn test_version_flag() {
    let output = Command::new(cache_sweep_bin())
        .arg("--version")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("cache-sweep"));
}

#[test]
fn test_json_output_empty_dir() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

    let output = Command::new(cache_sweep_bin())
        .arg("--json")
        .arg("-d")
        .arg(temp_dir.path())
        .arg("-t")
        .arg("node_modules")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Parse JSON
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("Invalid JSON");
    assert_eq!(json["version"], 1);
    assert!(json["results"].as_array().unwrap().is_empty());
}

#[test]
fn test_json_finds_target() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

    // Create a node_modules directory
    let node_modules = temp_dir.path().join("project").join("node_modules");
    fs::create_dir_all(&node_modules).expect("Failed to create dirs");

    // Add a file inside
    fs::write(node_modules.join("test.txt"), "test").expect("Failed to write file");

    let output = Command::new(cache_sweep_bin())
        .arg("--json")
        .arg("-d")
        .arg(temp_dir.path())
        .arg("-t")
        .arg("node_modules")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    let json: serde_json::Value = serde_json::from_str(&stdout).expect("Invalid JSON");
    let results = json["results"].as_array().unwrap();
    assert_eq!(results.len(), 1);
    assert!(results[0]["path"]
        .as_str()
        .unwrap()
        .contains("node_modules"));
}

#[test]
fn test_exclude_works() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

    // Create multiple target directories
    let nm1 = temp_dir.path().join("proj1").join("node_modules");
    let nm2 = temp_dir.path().join("proj2").join("node_modules");
    fs::create_dir_all(&nm1).expect("Failed to create dirs");
    fs::create_dir_all(&nm2).expect("Failed to create dirs");

    let output = Command::new(cache_sweep_bin())
        .arg("--json")
        .arg("-d")
        .arg(temp_dir.path())
        .arg("-t")
        .arg("node_modules")
        .arg("-E")
        .arg("proj1")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    let json: serde_json::Value = serde_json::from_str(&stdout).expect("Invalid JSON");
    let results = json["results"].as_array().unwrap();
    // Only proj2 should be found
    assert_eq!(results.len(), 1);
    assert!(results[0]["path"].as_str().unwrap().contains("proj2"));
}

#[test]
fn test_dry_run_does_not_delete() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

    let node_modules = temp_dir.path().join("project").join("node_modules");
    fs::create_dir_all(&node_modules).expect("Failed to create dirs");
    fs::write(node_modules.join("test.txt"), "test").expect("Failed to write file");

    let output = Command::new(cache_sweep_bin())
        .arg("--json")
        .arg("-d")
        .arg(temp_dir.path())
        .arg("-t")
        .arg("node_modules")
        .arg("--delete-all")
        .arg("--dry-run")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());

    // Directory should still exist
    assert!(node_modules.exists());
}

#[test]
fn test_multiple_profiles() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

    // Create node and python targets
    let node = temp_dir.path().join("proj").join("node_modules");
    let python = temp_dir.path().join("proj").join("__pycache__");
    fs::create_dir_all(&node).expect("Failed to create dirs");
    fs::create_dir_all(&python).expect("Failed to create dirs");

    let output = Command::new(cache_sweep_bin())
        .arg("--json")
        .arg("-d")
        .arg(temp_dir.path())
        .arg("-p")
        .arg("node,python")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    let json: serde_json::Value = serde_json::from_str(&stdout).expect("Invalid JSON");
    let results = json["results"].as_array().unwrap();
    assert_eq!(results.len(), 2);
}

#[test]
fn test_sort_flag_size() {
    let output = Command::new(cache_sweep_bin())
        .arg("--json")
        .arg("-d")
        .arg("/tmp")
        .arg("-t")
        .arg("nonexistent_target_xyz")
        .arg("-s")
        .arg("size")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
}

#[test]
fn test_sort_flag_path() {
    let output = Command::new(cache_sweep_bin())
        .arg("--json")
        .arg("-d")
        .arg("/tmp")
        .arg("-t")
        .arg("nonexistent_target_xyz")
        .arg("-s")
        .arg("path")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
}

#[test]
fn test_sort_flag_age() {
    let output = Command::new(cache_sweep_bin())
        .arg("--json")
        .arg("-d")
        .arg("/tmp")
        .arg("-t")
        .arg("nonexistent_target_xyz")
        .arg("-s")
        .arg("age")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
}

#[test]
fn test_sort_flag_invalid_defaults_to_size() {
    // Invalid sort value should not crash, falls back to size
    let output = Command::new(cache_sweep_bin())
        .arg("--json")
        .arg("-d")
        .arg("/tmp")
        .arg("-t")
        .arg("nonexistent_target_xyz")
        .arg("-s")
        .arg("invalid_sort_value")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
}

#[test]
fn test_profiles_and_targets_conflict() {
    let output = Command::new(cache_sweep_bin())
        .arg("--profiles")
        .arg("node")
        .arg("--targets")
        .arg("node_modules")
        .output()
        .expect("Failed to execute command");

    // Should fail due to conflict
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("cannot be used with"));
}

#[test]
fn test_json_stream_output() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

    // Create a node_modules directory
    let node_modules = temp_dir.path().join("project").join("node_modules");
    fs::create_dir_all(&node_modules).expect("Failed to create dirs");
    fs::write(node_modules.join("test.txt"), "test").expect("Failed to write file");

    let output = Command::new(cache_sweep_bin())
        .arg("--json-stream")
        .arg("-d")
        .arg(temp_dir.path())
        .arg("-t")
        .arg("node_modules")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Each line should be valid JSON
    for line in stdout.lines() {
        let json: serde_json::Value = serde_json::from_str(line).expect("Invalid JSON line");
        assert!(json["path"].as_str().unwrap().contains("node_modules"));
        assert!(json.get("size").is_some());
    }
}

#[test]
fn test_json_stream_empty_dir() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

    let output = Command::new(cache_sweep_bin())
        .arg("--json-stream")
        .arg("-d")
        .arg(temp_dir.path())
        .arg("-t")
        .arg("node_modules")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // No output for empty results
    assert!(stdout.trim().is_empty());
}

#[test]
fn test_delete_all_actually_deletes() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

    let node_modules = temp_dir.path().join("project").join("node_modules");
    fs::create_dir_all(&node_modules).expect("Failed to create dirs");
    fs::write(node_modules.join("test.txt"), "test").expect("Failed to write file");

    assert!(node_modules.exists());

    let output = Command::new(cache_sweep_bin())
        .arg("--json")
        .arg("-d")
        .arg(temp_dir.path())
        .arg("-t")
        .arg("node_modules")
        .arg("--delete-all")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());

    // Directory should be deleted
    assert!(!node_modules.exists());

    // JSON should show deleted: true
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("Invalid JSON");
    let results = json["results"].as_array().unwrap();
    assert_eq!(results[0]["deleted"], true);
}
