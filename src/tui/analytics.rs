use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::profiles::PROFILES;

/// Aggregated analytics data, updated in real-time during scan
#[derive(Debug, Default)]
pub struct AnalyticsData {
    /// Stats per target type (e.g., `node_modules`, `.next`)
    pub by_target: HashMap<String, TargetStats>,

    /// Stats per profile (e.g., "node", "python")
    pub by_profile: HashMap<String, ProfileStats>,

    /// Top N largest items by size
    pub top_largest: Vec<LargestItem>,
    pub top_largest_limit: usize,

    /// Scan metrics
    pub scan_start_time: Option<std::time::Instant>,
    pub scan_complete: bool,
    pub sizes_complete: bool,
    pub final_elapsed_secs: Option<f64>,
}

#[derive(Debug, Default, Clone)]
pub struct TargetStats {
    pub name: String,
    pub count: usize,
    pub total_size: u64,
    pub size_known_count: usize,
}

#[derive(Debug, Default, Clone)]
pub struct ProfileStats {
    pub name: String,
    pub count: usize,
    pub total_size: u64,
}

#[derive(Debug, Clone)]
pub struct LargestItem {
    pub path: PathBuf,
    pub size: u64,
    #[allow(dead_code)] // For future filtering by target type
    pub target_type: String,
}

impl AnalyticsData {
    pub fn new() -> Self {
        Self {
            top_largest_limit: 5,
            scan_start_time: Some(std::time::Instant::now()),
            ..Default::default()
        }
    }

    /// Update analytics when a new scan result arrives
    pub fn record_result(&mut self, path: &Path, size: Option<u64>) {
        let target_name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        let profile_name = Self::profile_for_target(&target_name);

        // Update target stats
        let target = self
            .by_target
            .entry(target_name.clone())
            .or_insert_with(|| TargetStats {
                name: target_name.clone(),
                ..Default::default()
            });
        target.count += 1;
        if let Some(s) = size {
            target.total_size += s;
            target.size_known_count += 1;
        }

        // Update profile stats
        let profile = self
            .by_profile
            .entry(profile_name.clone())
            .or_insert_with(|| ProfileStats {
                name: profile_name,
                ..Default::default()
            });
        profile.count += 1;
        if let Some(s) = size {
            profile.total_size += s;
        }

        // Update top largest (if size known)
        if let Some(s) = size {
            self.maybe_insert_largest(path.to_path_buf(), s, target_name);
        }
    }

    /// Update size for existing result (called when async size calculation completes)
    pub fn update_size(&mut self, path: &Path, old_size: Option<u64>, new_size: u64) {
        let target_name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        let profile_name = Self::profile_for_target(&target_name);

        // Update target stats (only add if size wasn't already known)
        if let Some(target) = self.by_target.get_mut(&target_name) {
            if old_size.is_none() {
                target.total_size += new_size;
                target.size_known_count += 1;
            }
        }

        // Update profile stats
        if let Some(profile) = self.by_profile.get_mut(&profile_name) {
            if old_size.is_none() {
                profile.total_size += new_size;
            }
        }

        // Update top largest
        self.maybe_insert_largest(path.to_path_buf(), new_size, target_name);
    }

    fn maybe_insert_largest(&mut self, path: PathBuf, size: u64, target_type: String) {
        // Check if this path is already in the list
        if let Some(existing) = self.top_largest.iter_mut().find(|item| item.path == path) {
            existing.size = size;
            // Re-sort after updating
            self.top_largest.sort_by(|a, b| b.size.cmp(&a.size));
            return;
        }

        let item = LargestItem {
            path,
            size,
            target_type,
        };

        // Insert in sorted position
        let pos = self
            .top_largest
            .iter()
            .position(|x| x.size < size)
            .unwrap_or(self.top_largest.len());
        if pos < self.top_largest_limit {
            self.top_largest.insert(pos, item);
            if self.top_largest.len() > self.top_largest_limit {
                self.top_largest.pop();
            }
        }
    }

    /// Reverse lookup: find which profile contains this target
    fn profile_for_target(target: &str) -> String {
        for profile in PROFILES.values() {
            if profile.targets.contains(&target) {
                return profile.name.to_string();
            }
        }
        "other".to_string()
    }

    /// Get results per second rate (0 when all work complete)
    #[allow(clippy::cast_precision_loss)] // Precision loss acceptable for display
    pub fn results_rate(&self) -> f64 {
        if self.sizes_complete {
            return 0.0;
        }
        let elapsed = self.elapsed_secs();
        if elapsed > 0.0 {
            self.total_count() as f64 / elapsed
        } else {
            0.0
        }
    }

    /// Get elapsed scan time in seconds (frozen when sizes complete)
    pub fn elapsed_secs(&self) -> f64 {
        // If sizes complete, use frozen time
        if self.sizes_complete {
            return self.final_elapsed_secs.unwrap_or(0.0);
        }
        // During work, compute live
        self.scan_start_time
            .map_or(0.0, |start| start.elapsed().as_secs_f64())
    }

    /// Get targets sorted by size (descending)
    pub fn targets_by_size(&self) -> Vec<&TargetStats> {
        let mut targets: Vec<_> = self.by_target.values().collect();
        targets.sort_by(|a, b| b.total_size.cmp(&a.total_size));
        targets
    }

    /// Get profiles sorted by size (descending)
    pub fn profiles_by_size(&self) -> Vec<&ProfileStats> {
        let mut profiles: Vec<_> = self.by_profile.values().collect();
        profiles.sort_by(|a, b| b.total_size.cmp(&a.total_size));
        profiles
    }

    /// Calculate total count across all targets
    pub fn total_count(&self) -> usize {
        self.by_target.values().map(|t| t.count).sum()
    }

    /// Calculate total size across all targets
    pub fn total_size(&self) -> u64 {
        self.by_target.values().map(|t| t.total_size).sum()
    }

    /// Mark scan (phase 1) as complete
    #[allow(clippy::missing_const_for_fn)] // &mut self methods can't be const
    pub fn mark_scan_complete(&mut self) {
        self.scan_complete = true;
    }

    /// Mark size calculation (phase 2) as complete - freezes timer
    pub fn mark_sizes_complete(&mut self) {
        self.sizes_complete = true;
        self.final_elapsed_secs = Some(
            self.scan_start_time
                .map_or(0.0, |start| start.elapsed().as_secs_f64()),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // === new() tests ===

    #[test]
    fn test_new_initializes_defaults() {
        let analytics = AnalyticsData::new();
        assert_eq!(analytics.top_largest_limit, 5);
        assert!(analytics.scan_start_time.is_some());
        assert!(!analytics.scan_complete);
        assert!(!analytics.sizes_complete);
        assert!(analytics.final_elapsed_secs.is_none());
        assert!(analytics.by_target.is_empty());
        assert!(analytics.by_profile.is_empty());
        assert!(analytics.top_largest.is_empty());
    }

    // === profile_for_target() tests ===

    #[test]
    fn test_profile_for_target_node_modules() {
        assert_eq!(AnalyticsData::profile_for_target("node_modules"), "node");
    }

    #[test]
    fn test_profile_for_target_next() {
        assert_eq!(AnalyticsData::profile_for_target(".next"), "node");
    }

    #[test]
    fn test_profile_for_target_venv() {
        assert_eq!(AnalyticsData::profile_for_target(".venv"), "python");
    }

    #[test]
    fn test_profile_for_target_pycache() {
        assert_eq!(AnalyticsData::profile_for_target("__pycache__"), "python");
    }

    #[test]
    fn test_profile_for_target_unknown() {
        assert_eq!(AnalyticsData::profile_for_target("unknown_dir"), "other");
    }

    // === record_result() tests ===

    #[test]
    fn test_record_result_creates_target_stats() {
        let mut analytics = AnalyticsData::new();
        let path = PathBuf::from("/project/node_modules");

        analytics.record_result(&path, Some(1000));

        assert_eq!(analytics.by_target.len(), 1);
        let target = analytics.by_target.get("node_modules").unwrap();
        assert_eq!(target.name, "node_modules");
        assert_eq!(target.count, 1);
        assert_eq!(target.total_size, 1000);
        assert_eq!(target.size_known_count, 1);
    }

    #[test]
    fn test_record_result_creates_profile_stats() {
        let mut analytics = AnalyticsData::new();
        let path = PathBuf::from("/project/node_modules");

        analytics.record_result(&path, Some(1000));

        assert_eq!(analytics.by_profile.len(), 1);
        let profile = analytics.by_profile.get("node").unwrap();
        assert_eq!(profile.name, "node");
        assert_eq!(profile.count, 1);
        assert_eq!(profile.total_size, 1000);
    }

    #[test]
    fn test_record_result_without_size() {
        let mut analytics = AnalyticsData::new();
        let path = PathBuf::from("/project/node_modules");

        analytics.record_result(&path, None);

        let target = analytics.by_target.get("node_modules").unwrap();
        assert_eq!(target.count, 1);
        assert_eq!(target.total_size, 0);
        assert_eq!(target.size_known_count, 0);
    }

    #[test]
    fn test_record_result_multiple_same_target() {
        let mut analytics = AnalyticsData::new();

        analytics.record_result(&PathBuf::from("/a/node_modules"), Some(100));
        analytics.record_result(&PathBuf::from("/b/node_modules"), Some(200));
        analytics.record_result(&PathBuf::from("/c/node_modules"), Some(300));

        let target = analytics.by_target.get("node_modules").unwrap();
        assert_eq!(target.count, 3);
        assert_eq!(target.total_size, 600);
    }

    #[test]
    fn test_record_result_multiple_profiles() {
        let mut analytics = AnalyticsData::new();

        analytics.record_result(&PathBuf::from("/a/node_modules"), Some(100));
        analytics.record_result(&PathBuf::from("/b/.venv"), Some(200));

        assert_eq!(analytics.by_profile.len(), 2);
        assert!(analytics.by_profile.contains_key("node"));
        assert!(analytics.by_profile.contains_key("python"));
    }

    #[test]
    fn test_record_result_adds_to_top_largest() {
        let mut analytics = AnalyticsData::new();

        analytics.record_result(&PathBuf::from("/a/node_modules"), Some(1000));

        assert_eq!(analytics.top_largest.len(), 1);
        assert_eq!(analytics.top_largest[0].size, 1000);
    }

    // === update_size() tests ===

    #[test]
    fn test_update_size_when_old_size_none() {
        let mut analytics = AnalyticsData::new();
        let path = PathBuf::from("/project/node_modules");

        // First record without size
        analytics.record_result(&path, None);
        assert_eq!(
            analytics.by_target.get("node_modules").unwrap().total_size,
            0
        );

        // Then update with size
        analytics.update_size(&path, None, 5000);
        assert_eq!(
            analytics.by_target.get("node_modules").unwrap().total_size,
            5000
        );
        assert_eq!(
            analytics
                .by_target
                .get("node_modules")
                .unwrap()
                .size_known_count,
            1
        );
    }

    #[test]
    fn test_update_size_skips_when_old_size_some() {
        let mut analytics = AnalyticsData::new();
        let path = PathBuf::from("/project/node_modules");

        // Record with size
        analytics.record_result(&path, Some(1000));

        // Update should not double-count
        analytics.update_size(&path, Some(1000), 2000);
        assert_eq!(
            analytics.by_target.get("node_modules").unwrap().total_size,
            1000
        );
    }

    #[test]
    fn test_update_size_updates_top_largest() {
        let mut analytics = AnalyticsData::new();
        let path = PathBuf::from("/project/node_modules");

        analytics.record_result(&path, Some(100));
        assert_eq!(analytics.top_largest[0].size, 100);

        analytics.update_size(&path, Some(100), 5000);
        assert_eq!(analytics.top_largest[0].size, 5000);
    }

    // === maybe_insert_largest() tests ===

    #[test]
    fn test_top_largest_sorted_descending() {
        let mut analytics = AnalyticsData::new();

        analytics.record_result(&PathBuf::from("/a/node_modules"), Some(100));
        analytics.record_result(&PathBuf::from("/b/node_modules"), Some(500));
        analytics.record_result(&PathBuf::from("/c/node_modules"), Some(200));

        assert_eq!(analytics.top_largest[0].size, 500);
        assert_eq!(analytics.top_largest[1].size, 200);
        assert_eq!(analytics.top_largest[2].size, 100);
    }

    #[test]
    fn test_top_largest_respects_limit() {
        let mut analytics = AnalyticsData::new();
        analytics.top_largest_limit = 3;

        for i in 0..10 {
            analytics.record_result(
                &PathBuf::from(format!("/project{i}/node_modules")),
                Some(i * 100),
            );
        }

        assert_eq!(analytics.top_largest.len(), 3);
        assert_eq!(analytics.top_largest[0].size, 900);
        assert_eq!(analytics.top_largest[1].size, 800);
        assert_eq!(analytics.top_largest[2].size, 700);
    }

    #[test]
    fn test_top_largest_updates_existing_path() {
        let mut analytics = AnalyticsData::new();
        let path = PathBuf::from("/project/node_modules");

        analytics.record_result(&path, Some(100));
        analytics.record_result(&PathBuf::from("/other/node_modules"), Some(50));

        // Update the first path's size
        analytics.maybe_insert_largest(path.clone(), 200, "node_modules".to_string());

        assert_eq!(analytics.top_largest.len(), 2);
        assert_eq!(analytics.top_largest[0].size, 200);
        assert_eq!(analytics.top_largest[0].path, path);
    }

    // === total_count() / total_size() tests ===

    #[test]
    fn test_total_count() {
        let mut analytics = AnalyticsData::new();

        analytics.record_result(&PathBuf::from("/a/node_modules"), Some(100));
        analytics.record_result(&PathBuf::from("/b/.venv"), Some(200));
        analytics.record_result(&PathBuf::from("/c/node_modules"), Some(300));

        assert_eq!(analytics.total_count(), 3);
    }

    #[test]
    fn test_total_size() {
        let mut analytics = AnalyticsData::new();

        analytics.record_result(&PathBuf::from("/a/node_modules"), Some(100));
        analytics.record_result(&PathBuf::from("/b/.venv"), Some(200));
        analytics.record_result(&PathBuf::from("/c/node_modules"), Some(300));

        assert_eq!(analytics.total_size(), 600);
    }

    // === targets_by_size() / profiles_by_size() tests ===

    #[test]
    fn test_targets_by_size_sorted() {
        let mut analytics = AnalyticsData::new();

        analytics.record_result(&PathBuf::from("/a/node_modules"), Some(100));
        analytics.record_result(&PathBuf::from("/b/.next"), Some(500));
        analytics.record_result(&PathBuf::from("/c/.venv"), Some(200));

        let targets = analytics.targets_by_size();
        assert_eq!(targets[0].total_size, 500);
        assert_eq!(targets[1].total_size, 200);
        assert_eq!(targets[2].total_size, 100);
    }

    #[test]
    fn test_profiles_by_size_sorted() {
        let mut analytics = AnalyticsData::new();

        // node: 100, python: 700
        analytics.record_result(&PathBuf::from("/a/node_modules"), Some(100));
        analytics.record_result(&PathBuf::from("/b/.venv"), Some(200));
        analytics.record_result(&PathBuf::from("/c/__pycache__"), Some(500));

        let profiles = analytics.profiles_by_size();
        assert_eq!(profiles[0].name, "python");
        assert_eq!(profiles[0].total_size, 700);
        assert_eq!(profiles[1].name, "node");
        assert_eq!(profiles[1].total_size, 100);
    }

    // === mark_scan_complete() / mark_sizes_complete() tests ===

    #[test]
    fn test_mark_scan_complete() {
        let mut analytics = AnalyticsData::new();
        assert!(!analytics.scan_complete);

        analytics.mark_scan_complete();

        assert!(analytics.scan_complete);
        assert!(!analytics.sizes_complete); // sizes not complete yet
    }

    #[test]
    fn test_mark_sizes_complete_freezes_time() {
        let mut analytics = AnalyticsData::new();

        // Wait a tiny bit so elapsed > 0
        std::thread::sleep(std::time::Duration::from_millis(10));

        analytics.mark_sizes_complete();

        assert!(analytics.sizes_complete);
        assert!(analytics.final_elapsed_secs.is_some());
        assert!(analytics.final_elapsed_secs.unwrap() > 0.0);
    }

    // === elapsed_secs() tests ===

    #[test]
    fn test_elapsed_secs_live_during_scan() {
        let analytics = AnalyticsData::new();

        std::thread::sleep(std::time::Duration::from_millis(10));

        let elapsed = analytics.elapsed_secs();
        assert!(elapsed >= 0.01); // At least 10ms
    }

    #[test]
    fn test_elapsed_secs_frozen_when_complete() {
        let mut analytics = AnalyticsData::new();

        std::thread::sleep(std::time::Duration::from_millis(20));
        analytics.mark_sizes_complete();
        let frozen = analytics.elapsed_secs();

        std::thread::sleep(std::time::Duration::from_millis(20));
        let still_frozen = analytics.elapsed_secs();

        // Should be the same (frozen)
        assert!((frozen - still_frozen).abs() < 0.001);
    }

    // === results_rate() tests ===

    #[test]
    fn test_results_rate_zero_when_complete() {
        let mut analytics = AnalyticsData::new();
        analytics.record_result(&PathBuf::from("/a/node_modules"), Some(100));
        analytics.mark_sizes_complete();

        assert_eq!(analytics.results_rate(), 0.0);
    }

    #[test]
    fn test_results_rate_positive_during_scan() {
        let mut analytics = AnalyticsData::new();

        // Add some results
        for i in 0..10 {
            analytics.record_result(
                &PathBuf::from(format!("/project{i}/node_modules")),
                Some(100),
            );
        }

        std::thread::sleep(std::time::Duration::from_millis(10));

        let rate = analytics.results_rate();
        assert!(rate > 0.0);
    }
}
