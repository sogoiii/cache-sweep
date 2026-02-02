use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::time::SystemTime;

use crate::risk::{analyze_risk, RiskAnalysis};
use crate::scanner::ScanResult;

use super::analytics::AnalyticsData;

/// Extracts the target folder name from a path (e.g., `node_modules` from `/foo/bar/node_modules`)
fn extract_target_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default()
}

/// A group of results sharing the same target folder name (for tab filtering)
#[derive(Debug, Clone)]
pub struct TargetGroup {
    pub name: String,        // "node_modules", ".venv", etc.
    pub indices: Vec<usize>, // indices into results vec (stable)
    pub total_size: u64,     // sum of sizes for this group
    pub count: usize,        // number of non-deleted items
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Panel {
    Results,
    Info,
    Analytics,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Search,
    MultiSelect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortOrder {
    #[default]
    Size,
    Path,
    Age,
}

impl SortOrder {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "path" => Self::Path,
            "age" => Self::Age,
            _ => Self::Size,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ResultItem {
    pub scan_result: ScanResult,
    pub risk: RiskAnalysis,
    pub is_selected: bool,
    pub is_deleting: bool,
    pub is_deleted: bool,
}

impl ResultItem {
    pub fn from_scan_result(mut result: ScanResult) -> Self {
        let risk = analyze_risk(&result.path);
        result.is_sensitive = risk.is_sensitive;
        Self {
            scan_result: result,
            risk,
            is_selected: false,
            is_deleting: false,
            is_deleted: false,
        }
    }
}

#[allow(clippy::struct_excessive_bools)] // TUI state naturally tracks multiple boolean flags
pub struct App {
    pub results: Vec<ResultItem>,
    pub filtered_indices: Vec<usize>,
    pub cursor: usize,
    pub scroll_offset: usize,
    pub panel: Panel,
    pub mode: Mode,
    pub sort_order: SortOrder,
    pub search_query: String,
    pub needs_filter: bool,
    pub needs_sort: bool,
    pub scanning: bool,
    pub total_size: u64,
    pub freed_size: u64,
    pub errors: Vec<String>,
    pub selected_indices: HashSet<usize>,
    pub visible_height: usize,
    pub exclude_sensitive: bool,
    pub sizes_calculated: usize,
    pub spinner_tick: usize,
    pub sort_flash: usize,
    // Analytics
    pub analytics: AnalyticsData,
    pub analytics_scroll: usize,
    // Tabs (filtering by target type)
    pub target_groups: Vec<TargetGroup>,
    pub active_tab: usize,          // 0 = "All", 1+ = specific groups
    pub tab_scroll_offset: usize,   // first visible group index in scrollable area
    pub visible_group_count: usize, // how many groups fit (set by UI)
}

impl App {
    pub fn new(exclude_sensitive: bool, sort_order: SortOrder) -> Self {
        Self {
            results: Vec::new(),
            filtered_indices: Vec::new(),
            cursor: 0,
            scroll_offset: 0,
            panel: Panel::Results,
            mode: Mode::Normal,
            sort_order,
            search_query: String::new(),
            needs_filter: false,
            needs_sort: false,
            scanning: true,
            total_size: 0,
            freed_size: 0,
            errors: Vec::new(),
            selected_indices: HashSet::new(),
            visible_height: 20,
            exclude_sensitive,
            sizes_calculated: 0,
            spinner_tick: 0,
            sort_flash: 0,
            analytics: AnalyticsData::new(),
            analytics_scroll: 0,
            target_groups: Vec::new(),
            active_tab: 0,          // 0 = "All" tab (always first)
            tab_scroll_offset: 0,   // first visible group in scrollable area
            visible_group_count: 5, // default, updated by UI on render
        }
    }

    pub fn add_results(&mut self, results: Vec<ScanResult>) {
        for result in results {
            let item = ResultItem::from_scan_result(result);

            // Skip sensitive if excluded
            if self.exclude_sensitive && item.risk.is_sensitive {
                continue;
            }

            // Record in analytics
            self.analytics
                .record_result(&item.scan_result.path, item.scan_result.size);

            self.results.push(item);
        }
        // Rebuild display indices (filter + sort)
        self.rebuild_display_indices();
    }

    pub fn update_size(&mut self, index: usize, size: u64, file_count: u64) {
        if let Some(item) = self.results.get_mut(index) {
            let old_size = item.scan_result.size;
            item.scan_result.size = Some(size);
            item.scan_result.file_count = Some(file_count);
            self.total_size += size; // O(1) incremental update
            self.sizes_calculated += 1;
            self.needs_sort = true; // Debounce: sort on tick instead of every update

            // Update analytics
            self.analytics
                .update_size(&item.scan_result.path.clone(), old_size, size);
        }
    }

    pub fn on_tick(&mut self) {
        self.spinner_tick = self.spinner_tick.wrapping_add(1);
        self.sort_flash = self.sort_flash.saturating_sub(1);

        // Handle deferred sort (debounced from size updates)
        if self.needs_sort {
            self.rebuild_display_indices();
            self.needs_sort = false;
        }

        // Handle deferred filter (e.g., after deletion)
        if self.needs_filter {
            self.rebuild_display_indices();
            self.needs_filter = false;
        }
    }

    /// Rebuilds target groups for tab filtering.
    /// Called when results change (add, delete, size update).
    /// Tab layout: [All] [group1] [group2] ... (All is always index 0)
    fn rebuild_target_groups(&mut self) {
        let mut groups: HashMap<String, TargetGroup> = HashMap::new();

        for (idx, item) in self.results.iter().enumerate() {
            if item.is_deleted {
                continue;
            }

            let target_name = extract_target_name(&item.scan_result.path);
            let size = item.scan_result.size.unwrap_or(0);

            let group = groups
                .entry(target_name.clone())
                .or_insert_with(|| TargetGroup {
                    name: target_name,
                    indices: Vec::new(),
                    total_size: 0,
                    count: 0,
                });
            group.indices.push(idx);
            group.total_size += size;
            group.count += 1;
        }

        // Sort groups by total size descending
        let mut groups: Vec<TargetGroup> = groups.into_values().collect();
        groups.sort_by(|a, b| b.total_size.cmp(&a.total_size));

        self.target_groups = groups;

        // Clamp active_tab to valid range (0 = All, 1..=len = specific groups)
        let max_tab = self.target_groups.len();
        if self.active_tab > max_tab {
            self.active_tab = 0; // Reset to "All"
        }
    }

    /// Rebuilds `filtered_indices` by filtering then sorting.
    /// Results vec is NEVER reordered - indices point into stable positions.
    fn rebuild_display_indices(&mut self) {
        // Step 0: Rebuild target groups first
        self.rebuild_target_groups();

        // Step 1: Get base indices from active tab
        // Tab layout: 0 = "All", 1..=len = specific groups
        let base_indices: Vec<usize> = if self.active_tab == 0 || self.target_groups.is_empty() {
            // "All" tab - use all non-deleted indices
            (0..self.results.len())
                .filter(|&i| !self.results[i].is_deleted)
                .collect()
        } else {
            // Specific target tab (index 1 = first group, etc.)
            self.target_groups[self.active_tab - 1].indices.clone()
        };

        // Step 2: Apply search filter
        let query = self.search_query.to_lowercase();
        self.filtered_indices = base_indices
            .into_iter()
            .filter(|&i| {
                let item = &self.results[i];
                self.search_query.is_empty()
                    || item
                        .scan_result
                        .path
                        .to_string_lossy()
                        .to_lowercase()
                        .contains(&query)
            })
            .collect();

        // Step 2: Sort the indices (not the results!) based on sort order
        match self.sort_order {
            SortOrder::Size => {
                self.filtered_indices.sort_by(|&a, &b| {
                    let size_a = self.results[a].scan_result.size.unwrap_or(0);
                    let size_b = self.results[b].scan_result.size.unwrap_or(0);
                    size_b.cmp(&size_a) // Descending
                });
            }
            SortOrder::Path => {
                self.filtered_indices.sort_by(|&a, &b| {
                    self.results[a]
                        .scan_result
                        .path
                        .cmp(&self.results[b].scan_result.path)
                });
            }
            SortOrder::Age => {
                self.filtered_indices.sort_by(|&a, &b| {
                    let time_a = self.results[a]
                        .scan_result
                        .modified
                        .unwrap_or(SystemTime::UNIX_EPOCH);
                    let time_b = self.results[b]
                        .scan_result
                        .modified
                        .unwrap_or(SystemTime::UNIX_EPOCH);
                    time_a.cmp(&time_b) // Oldest first
                });
            }
        }

        // Adjust cursor if out of bounds
        if self.cursor >= self.filtered_indices.len() && !self.filtered_indices.is_empty() {
            self.cursor = self.filtered_indices.len() - 1;
        }
    }

    /// Move to next tab (wraps around)
    /// Tab layout: 0 = "All", 1..=len = specific groups
    pub fn next_tab(&mut self) {
        let num_tabs = self.target_groups.len() + 1; // +1 for "All"
        self.active_tab = (self.active_tab + 1) % num_tabs;
        self.cursor = 0;
        self.scroll_offset = 0;
        self.adjust_tab_scroll();
        self.rebuild_display_indices();
    }

    /// Move to previous tab (wraps around)
    pub fn prev_tab(&mut self) {
        let num_tabs = self.target_groups.len() + 1; // +1 for "All"
        self.active_tab = if self.active_tab == 0 {
            num_tabs - 1
        } else {
            self.active_tab - 1
        };
        self.cursor = 0;
        self.scroll_offset = 0;
        self.adjust_tab_scroll();
        self.rebuild_display_indices();
    }

    /// Adjusts `tab_scroll_offset` to keep `active_tab` visible
    pub fn adjust_tab_scroll(&mut self) {
        let visible_group_count = self.visible_group_count;
        if self.active_tab == 0 || self.target_groups.is_empty() {
            // On "All" tab - no need to scroll groups
            return;
        }

        // active_tab 1 = group index 0, active_tab 2 = group index 1, etc.
        let group_idx = self.active_tab - 1;

        // Scroll left if selected is before visible window
        if group_idx < self.tab_scroll_offset {
            self.tab_scroll_offset = group_idx;
        }

        // Scroll right if selected is past visible window
        if visible_group_count > 0 && group_idx >= self.tab_scroll_offset + visible_group_count {
            self.tab_scroll_offset = group_idx - visible_group_count + 1;
        }

        // Clamp to valid range
        let max_offset = self.target_groups.len().saturating_sub(visible_group_count);
        self.tab_scroll_offset = self.tab_scroll_offset.min(max_offset);
    }

    /// Returns subtotal size for the active tab
    /// Tab layout: 0 = "All", 1..=len = specific groups
    pub fn active_tab_subtotal(&self) -> u64 {
        if self.active_tab == 0 || self.target_groups.is_empty() {
            self.total_size
        } else {
            self.target_groups[self.active_tab - 1].total_size
        }
    }

    pub fn visible_results(&self) -> Vec<(usize, &ResultItem)> {
        let mut results = Vec::with_capacity(self.visible_height);
        results.extend(
            self.filtered_indices
                .iter()
                .skip(self.scroll_offset)
                .take(self.visible_height)
                .filter_map(|&i| self.results.get(i).map(|item| (i, item))),
        );
        results
    }

    pub fn current_item(&self) -> Option<&ResultItem> {
        self.current_index().and_then(|i| self.results.get(i))
    }

    pub fn current_index(&self) -> Option<usize> {
        self.filtered_indices.get(self.cursor).copied()
    }

    pub fn move_cursor(&mut self, delta: isize) {
        let max_pos = self.filtered_indices.len().saturating_sub(1);
        self.cursor = if delta >= 0 {
            #[allow(clippy::cast_sign_loss)] // Checked: delta >= 0
            self.cursor.saturating_add(delta as usize).min(max_pos)
        } else {
            self.cursor.saturating_sub(delta.unsigned_abs())
        };
        self.adjust_scroll();
    }

    /// Move cursor by a full page (up or down).
    pub fn move_cursor_by_page(&mut self, down: bool) {
        let max_pos = self.filtered_indices.len().saturating_sub(1);
        self.cursor = if down {
            self.cursor.saturating_add(self.visible_height).min(max_pos)
        } else {
            self.cursor.saturating_sub(self.visible_height)
        };
        self.adjust_scroll();
    }

    #[allow(clippy::missing_const_for_fn)] // &mut self methods can't be const
    fn adjust_scroll(&mut self) {
        let max_len = self.filtered_indices.len();
        if max_len == 0 {
            self.scroll_offset = 0;
            return;
        }

        if self.cursor < self.scroll_offset {
            self.scroll_offset = self.cursor;
        } else if self.cursor + 2 >= self.scroll_offset + self.visible_height {
            // Keep cursor 2 rows above the bottom (accounts for UI chrome)
            self.scroll_offset = self.cursor + 3 - self.visible_height;
        }
    }

    pub fn toggle_selection(&mut self) {
        if let Some(idx) = self.current_index() {
            if self.selected_indices.contains(&idx) {
                self.selected_indices.remove(&idx);
                if let Some(item) = self.results.get_mut(idx) {
                    item.is_selected = false;
                }
            } else {
                self.selected_indices.insert(idx);
                if let Some(item) = self.results.get_mut(idx) {
                    item.is_selected = true;
                }
            }
        }
    }

    pub fn select_all(&mut self) {
        for &idx in &self.filtered_indices {
            self.selected_indices.insert(idx);
            if let Some(item) = self.results.get_mut(idx) {
                item.is_selected = true;
            }
        }
    }

    pub fn deselect_all(&mut self) {
        for idx in self.selected_indices.drain() {
            if let Some(item) = self.results.get_mut(idx) {
                item.is_selected = false;
            }
        }
    }

    pub fn mark_deleted(&mut self, index: usize, size_freed: u64) {
        if let Some(item) = self.results.get_mut(index) {
            item.is_deleted = true;
            item.is_deleting = false;
            self.freed_size += size_freed;
            self.selected_indices.remove(&index);
        }
        self.needs_filter = true;
        // Grouped display will be rebuilt via needs_filter -> rebuild_display_indices
    }

    pub fn mark_deleting(&mut self, index: usize) {
        if let Some(item) = self.results.get_mut(index) {
            item.is_deleting = true;
        }
    }

    pub fn scan_complete(&mut self) {
        self.scanning = false;
        self.analytics.mark_scan_complete();
        // Ensure display indices are rebuilt when scan finishes
        self.rebuild_display_indices();
    }

    /// Called when all size calculations are done
    pub fn sizes_complete(&mut self) {
        self.analytics.mark_sizes_complete();
    }

    pub fn add_error(&mut self, error: String) {
        self.errors.push(error);
    }

    pub fn apply_sort_and_filter(&mut self) {
        self.rebuild_display_indices();
        self.sort_flash = 5; // Brief yellow highlight (~500ms)
    }

    pub const fn sort_label(&self) -> &'static str {
        match self.sort_order {
            SortOrder::Size => "SIZE",
            SortOrder::Path => "PATH",
            SortOrder::Age => "AGE",
        }
    }

    pub const fn is_calculating_sizes(&self) -> bool {
        !self.results.is_empty() && self.sizes_calculated < self.results.len()
    }

    #[allow(dead_code)] // Kept for potential future use
    #[allow(clippy::cast_precision_loss)] // Precision loss acceptable for progress display
    pub fn size_progress(&self) -> f64 {
        if self.results.is_empty() {
            0.0
        } else {
            self.sizes_calculated as f64 / self.results.len() as f64
        }
    }

    #[allow(dead_code)] // Kept for potential future use
    pub fn spinner_char(&self) -> char {
        const SPINNER: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
        SPINNER[self.spinner_tick % SPINNER.len()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // Helper to create an App with N items for cursor tests
    fn app_with_items(item_count: usize, visible_height: usize) -> App {
        let mut app = App::new(false, SortOrder::Size);
        app.visible_height = visible_height;
        app.filtered_indices = (0..item_count).collect();
        app
    }

    // SortOrder tests
    #[test]
    fn test_sort_order_from_str_size() {
        assert_eq!(SortOrder::from_str("size"), SortOrder::Size);
        assert_eq!(SortOrder::from_str("SIZE"), SortOrder::Size);
        assert_eq!(SortOrder::from_str("Size"), SortOrder::Size);
    }

    #[test]
    fn test_sort_order_from_str_path() {
        assert_eq!(SortOrder::from_str("path"), SortOrder::Path);
        assert_eq!(SortOrder::from_str("PATH"), SortOrder::Path);
        assert_eq!(SortOrder::from_str("Path"), SortOrder::Path);
    }

    #[test]
    fn test_sort_order_from_str_age() {
        assert_eq!(SortOrder::from_str("age"), SortOrder::Age);
        assert_eq!(SortOrder::from_str("AGE"), SortOrder::Age);
        assert_eq!(SortOrder::from_str("Age"), SortOrder::Age);
    }

    #[test]
    fn test_sort_order_from_str_invalid_defaults_to_size() {
        assert_eq!(SortOrder::from_str("invalid"), SortOrder::Size);
        assert_eq!(SortOrder::from_str(""), SortOrder::Size);
        assert_eq!(SortOrder::from_str("foo"), SortOrder::Size);
    }

    // Cursor movement tests
    #[test]
    fn test_move_cursor_down_basic() {
        let mut app = app_with_items(50, 20);
        assert_eq!(app.cursor, 0);

        app.move_cursor(1);
        assert_eq!(app.cursor, 1);

        app.move_cursor(5);
        assert_eq!(app.cursor, 6);
    }

    #[test]
    fn test_move_cursor_up_basic() {
        let mut app = app_with_items(50, 20);
        app.cursor = 10;

        app.move_cursor(-1);
        assert_eq!(app.cursor, 9);

        app.move_cursor(-5);
        assert_eq!(app.cursor, 4);
    }

    #[test]
    fn test_move_cursor_does_not_go_negative() {
        let mut app = app_with_items(50, 20);
        app.cursor = 2;

        app.move_cursor(-10);
        assert_eq!(app.cursor, 0);
    }

    #[test]
    fn test_move_cursor_does_not_exceed_list_length() {
        let mut app = app_with_items(50, 20);
        app.cursor = 45;

        app.move_cursor(10);
        assert_eq!(app.cursor, 49); // Last item
    }

    #[test]
    fn test_move_cursor_empty_list() {
        let mut app = app_with_items(0, 20);

        app.move_cursor(1);
        assert_eq!(app.cursor, 0);

        app.move_cursor(-1);
        assert_eq!(app.cursor, 0);
    }

    // Scroll offset tests
    #[test]
    fn test_scroll_keeps_2_row_buffer_at_bottom() {
        let mut app = app_with_items(50, 20);
        // visible_height = 20, so cursor at position 17 (0-indexed) should trigger scroll
        // because 17 >= 0 + 20 - 2 = 18 is false, but 18 >= 18 is true

        // Move to position 17 - no scroll yet
        app.cursor = 17;
        app.move_cursor(0); // Trigger scroll check
        assert_eq!(app.scroll_offset, 0);

        // Move to position 18 - should trigger scroll
        app.move_cursor(1);
        assert_eq!(app.cursor, 18);
        // scroll_offset = 18 - 20 + 3 = 1
        assert_eq!(app.scroll_offset, 1);
    }

    #[test]
    fn test_scroll_down_maintains_buffer() {
        let mut app = app_with_items(50, 20);

        // Move cursor to trigger multiple scrolls
        for _ in 0..30 {
            app.move_cursor(1);
            // Cursor should always be at most visible_height - 3 from scroll_offset
            let visible_position = app.cursor - app.scroll_offset;
            assert!(
                visible_position <= app.visible_height - 3,
                "At cursor {}, scroll_offset {}, visible_position {} exceeds limit {}",
                app.cursor,
                app.scroll_offset,
                visible_position,
                app.visible_height - 3
            );
        }
    }

    #[test]
    fn test_scroll_up_adjusts_offset() {
        let mut app = app_with_items(50, 20);
        app.cursor = 25;
        app.scroll_offset = 10;

        // Move up past the visible area
        app.move_cursor(-20);
        assert_eq!(app.cursor, 5);
        // scroll_offset should adjust to show cursor
        assert_eq!(app.scroll_offset, 5);
    }

    #[test]
    fn test_scroll_up_cursor_at_top() {
        let mut app = app_with_items(50, 20);
        app.cursor = 10;
        app.scroll_offset = 10;

        // Move up one - cursor goes above scroll_offset
        app.move_cursor(-1);
        assert_eq!(app.cursor, 9);
        assert_eq!(app.scroll_offset, 9);
    }

    #[test]
    fn test_no_scroll_when_list_smaller_than_visible() {
        let mut app = app_with_items(10, 20);

        // Move to end of list
        app.move_cursor(15);
        assert_eq!(app.cursor, 9); // Last item
        assert_eq!(app.scroll_offset, 0); // No scroll needed
    }

    #[test]
    fn test_page_down() {
        let mut app = app_with_items(100, 20);

        // Page down moves by visible_height
        app.move_cursor(20);
        assert_eq!(app.cursor, 20);
        // Should have scrolled to maintain buffer
        assert!(app.scroll_offset > 0);
    }

    #[test]
    fn test_page_up() {
        let mut app = app_with_items(100, 20);
        app.cursor = 50;
        app.scroll_offset = 35;

        // Page up moves by -visible_height
        app.move_cursor(-20);
        assert_eq!(app.cursor, 30);
        // scroll_offset should adjust
        assert!(app.scroll_offset <= app.cursor);
    }

    #[test]
    fn test_cursor_visible_position_never_exceeds_buffer() {
        // Comprehensive test: navigate through entire list
        let mut app = app_with_items(100, 20);

        for _ in 0..99 {
            app.move_cursor(1);

            if app.scroll_offset > 0 || app.cursor >= app.visible_height - 2 {
                let visible_pos = app.cursor - app.scroll_offset;
                assert!(
                    visible_pos <= app.visible_height - 3,
                    "cursor {} scroll {} visible_pos {} exceeds {}",
                    app.cursor,
                    app.scroll_offset,
                    visible_pos,
                    app.visible_height - 3
                );
            }
        }
    }

    // === Stable index tests (regression prevention for size calculation bug) ===

    fn make_scan_result(path: &str, size: Option<u64>) -> ScanResult {
        ScanResult {
            path: PathBuf::from(path),
            size,
            file_count: None,
            modified: None,
            is_sensitive: false,
        }
    }

    #[test]
    fn test_results_order_stable_after_add() {
        let mut app = App::new(false, SortOrder::Size);

        // Add results with different sizes
        app.add_results(vec![
            make_scan_result("/a", Some(100)),
            make_scan_result("/b", Some(500)),
            make_scan_result("/c", Some(200)),
        ]);

        // Results should be in original order (append-only)
        assert_eq!(app.results[0].scan_result.path, PathBuf::from("/a"));
        assert_eq!(app.results[1].scan_result.path, PathBuf::from("/b"));
        assert_eq!(app.results[2].scan_result.path, PathBuf::from("/c"));
    }

    #[test]
    fn test_filtered_indices_sorted_not_results() {
        let mut app = App::new(false, SortOrder::Size);

        app.add_results(vec![
            make_scan_result("/small", Some(100)),
            make_scan_result("/large", Some(500)),
            make_scan_result("/medium", Some(200)),
        ]);

        // Results still in original order
        assert_eq!(app.results[0].scan_result.path, PathBuf::from("/small"));
        assert_eq!(app.results[1].scan_result.path, PathBuf::from("/large"));
        assert_eq!(app.results[2].scan_result.path, PathBuf::from("/medium"));

        // But filtered_indices should be sorted by size desc: [1, 2, 0]
        assert_eq!(app.filtered_indices, vec![1, 2, 0]);
    }

    #[test]
    fn test_update_size_uses_stable_index() {
        let mut app = App::new(false, SortOrder::Size);

        // Add results - index 0=/a, index 1=/b, index 2=/c
        app.add_results(vec![
            make_scan_result("/a", None),
            make_scan_result("/b", None),
            make_scan_result("/c", None),
        ]);

        // Simulate size calculation completing for index 1 (/b)
        app.update_size(1, 999, 10);

        // Index 1 should ALWAYS be /b, regardless of sorting
        assert_eq!(app.results[1].scan_result.path, PathBuf::from("/b"));
        assert_eq!(app.results[1].scan_result.size, Some(999));
    }

    #[test]
    fn test_update_size_correct_after_multiple_sorts() {
        let mut app = App::new(false, SortOrder::Size);

        app.add_results(vec![
            make_scan_result("/first", Some(100)),
            make_scan_result("/second", None), // No size yet
            make_scan_result("/third", Some(300)),
        ]);

        // Change sort order multiple times
        app.sort_order = SortOrder::Path;
        app.apply_sort_and_filter();
        app.sort_order = SortOrder::Size;
        app.apply_sort_and_filter();

        // Now update size for index 1 (should still be /second)
        app.update_size(1, 500, 5);

        // Verify /second got the update, not some other item
        assert_eq!(app.results[1].scan_result.path, PathBuf::from("/second"));
        assert_eq!(app.results[1].scan_result.size, Some(500));
    }

    #[test]
    fn test_visible_results_uses_filtered_order() {
        let mut app = App::new(false, SortOrder::Size);
        app.visible_height = 10;

        app.add_results(vec![
            make_scan_result("/small", Some(100)),
            make_scan_result("/large", Some(500)),
            make_scan_result("/medium", Some(200)),
        ]);

        let visible = app.visible_results();

        // Should be in sorted order: large, medium, small
        assert_eq!(visible[0].1.scan_result.path, PathBuf::from("/large"));
        assert_eq!(visible[1].1.scan_result.path, PathBuf::from("/medium"));
        assert_eq!(visible[2].1.scan_result.path, PathBuf::from("/small"));

        // But the indices returned should be the raw indices
        assert_eq!(visible[0].0, 1); // /large is at raw index 1
        assert_eq!(visible[1].0, 2); // /medium is at raw index 2
        assert_eq!(visible[2].0, 0); // /small is at raw index 0
    }

    // === Tab functionality tests ===

    #[test]
    fn test_target_groups_created_from_results() {
        let mut app = App::new(false, SortOrder::Size);

        app.add_results(vec![
            make_scan_result("/projects/a/node_modules", Some(100)),
            make_scan_result("/projects/b/node_modules", Some(200)),
            make_scan_result("/projects/c/.venv", Some(500)),
        ]);

        // Should have 2 groups: .venv and node_modules
        assert_eq!(app.target_groups.len(), 2);
    }

    #[test]
    fn test_target_groups_sorted_by_size_descending() {
        let mut app = App::new(false, SortOrder::Size);

        app.add_results(vec![
            make_scan_result("/a/node_modules", Some(100)),
            make_scan_result("/b/node_modules", Some(200)),
            make_scan_result("/c/.venv", Some(500)),
            make_scan_result("/d/target", Some(50)),
        ]);

        // Groups should be sorted by total size: .venv (500), node_modules (300), target (50)
        assert_eq!(app.target_groups[0].name, ".venv");
        assert_eq!(app.target_groups[0].total_size, 500);
        assert_eq!(app.target_groups[1].name, "node_modules");
        assert_eq!(app.target_groups[1].total_size, 300);
        assert_eq!(app.target_groups[2].name, "target");
        assert_eq!(app.target_groups[2].total_size, 50);
    }

    #[test]
    fn test_next_tab_wraps_around() {
        let mut app = App::new(false, SortOrder::Size);

        app.add_results(vec![
            make_scan_result("/a/node_modules", Some(100)),
            make_scan_result("/b/.venv", Some(200)),
        ]);

        // Start on "All" (tab 0)
        assert_eq!(app.active_tab, 0);

        // Tab through: All -> .venv -> node_modules -> All
        app.next_tab();
        assert_eq!(app.active_tab, 1); // .venv (largest)

        app.next_tab();
        assert_eq!(app.active_tab, 2); // node_modules

        app.next_tab();
        assert_eq!(app.active_tab, 0); // Back to All
    }

    #[test]
    fn test_prev_tab_wraps_around() {
        let mut app = App::new(false, SortOrder::Size);

        app.add_results(vec![
            make_scan_result("/a/node_modules", Some(100)),
            make_scan_result("/b/.venv", Some(200)),
        ]);

        // Start on "All" (tab 0)
        assert_eq!(app.active_tab, 0);

        // Prev from All wraps to last group
        app.prev_tab();
        assert_eq!(app.active_tab, 2); // node_modules (last)

        app.prev_tab();
        assert_eq!(app.active_tab, 1); // .venv

        app.prev_tab();
        assert_eq!(app.active_tab, 0); // Back to All
    }

    #[test]
    fn test_tab_filters_to_target_type() {
        let mut app = App::new(false, SortOrder::Size);

        app.add_results(vec![
            make_scan_result("/a/node_modules", Some(100)),
            make_scan_result("/b/node_modules", Some(200)),
            make_scan_result("/c/.venv", Some(500)),
        ]);

        // On "All" tab - should see all 3
        assert_eq!(app.active_tab, 0);
        assert_eq!(app.filtered_indices.len(), 3);

        // Switch to .venv tab (index 1, largest group)
        app.next_tab();
        assert_eq!(app.active_tab, 1);
        assert_eq!(app.filtered_indices.len(), 1);

        // Switch to node_modules tab (index 2)
        app.next_tab();
        assert_eq!(app.active_tab, 2);
        assert_eq!(app.filtered_indices.len(), 2);
    }

    #[test]
    fn test_active_tab_subtotal_all() {
        let mut app = App::new(false, SortOrder::Size);

        app.add_results(vec![
            make_scan_result("/a/node_modules", None),
            make_scan_result("/b/.venv", None),
        ]);

        // Simulate size calculations completing (this updates total_size)
        app.update_size(0, 100, 10);
        app.update_size(1, 200, 20);

        // On "All" tab - subtotal is total_size
        assert_eq!(app.active_tab, 0);
        assert_eq!(app.active_tab_subtotal(), app.total_size);
        assert_eq!(app.active_tab_subtotal(), 300);
    }

    #[test]
    fn test_active_tab_subtotal_specific_group() {
        let mut app = App::new(false, SortOrder::Size);

        app.add_results(vec![
            make_scan_result("/a/node_modules", Some(100)),
            make_scan_result("/b/node_modules", Some(150)),
            make_scan_result("/c/.venv", Some(500)),
        ]);

        // Switch to .venv tab
        app.next_tab();
        assert_eq!(app.active_tab_subtotal(), 500);

        // Switch to node_modules tab
        app.next_tab();
        assert_eq!(app.active_tab_subtotal(), 250);
    }

    #[test]
    fn test_tab_resets_cursor_on_switch() {
        let mut app = App::new(false, SortOrder::Size);
        app.visible_height = 10;

        app.add_results(vec![
            make_scan_result("/a/node_modules", Some(100)),
            make_scan_result("/b/node_modules", Some(200)),
            make_scan_result("/c/.venv", Some(500)),
        ]);

        // Move cursor
        app.cursor = 2;
        app.scroll_offset = 1;

        // Switch tab - cursor should reset
        app.next_tab();
        assert_eq!(app.cursor, 0);
        assert_eq!(app.scroll_offset, 0);
    }

    #[test]
    fn test_deleted_items_excluded_from_groups() {
        let mut app = App::new(false, SortOrder::Size);

        app.add_results(vec![
            make_scan_result("/a/node_modules", Some(100)),
            make_scan_result("/b/node_modules", Some(200)),
        ]);

        // Both in node_modules group
        assert_eq!(app.target_groups.len(), 1);
        assert_eq!(app.target_groups[0].count, 2);
        assert_eq!(app.target_groups[0].total_size, 300);

        // Delete one
        app.mark_deleted(0, 100);
        app.on_tick(); // Triggers rebuild

        // Group should now have 1 item
        assert_eq!(app.target_groups[0].count, 1);
        assert_eq!(app.target_groups[0].total_size, 200);
    }

    #[test]
    fn test_tab_scroll_offset_adjusts_for_selection() {
        let mut app = App::new(false, SortOrder::Size);
        app.visible_group_count = 2; // Only 2 groups visible at a time

        // Create 5 different target types
        app.add_results(vec![
            make_scan_result("/a/node_modules", Some(500)),
            make_scan_result("/b/.venv", Some(400)),
            make_scan_result("/c/target", Some(300)),
            make_scan_result("/d/.next", Some(200)),
            make_scan_result("/e/__pycache__", Some(100)),
        ]);

        // Start at All (tab 0), scroll offset should be 0
        assert_eq!(app.tab_scroll_offset, 0);

        // Navigate to tab 4 (.next) - should scroll
        app.active_tab = 4;
        app.adjust_tab_scroll();

        // Scroll offset should adjust to show tab 4
        // With visible_group_count=2, offset should be 2 to show tabs 3,4
        assert!(app.tab_scroll_offset >= 2);
    }
}
