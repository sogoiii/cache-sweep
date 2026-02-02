use std::collections::HashSet;
use std::time::SystemTime;

use crate::risk::{analyze_risk, RiskAnalysis};
use crate::scanner::ScanResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Panel {
    Results,
    Info,
    Options,
    Help,
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
            "path" => SortOrder::Path,
            "age" => SortOrder::Age,
            _ => SortOrder::Size,
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
        }
    }

    pub fn add_results(&mut self, results: Vec<ScanResult>) {
        for result in results {
            let item = ResultItem::from_scan_result(result);

            // Skip sensitive if excluded
            if self.exclude_sensitive && item.risk.is_sensitive {
                continue;
            }

            self.results.push(item);
        }
        // Sort immediately so results display in correct order
        self.sort_results();
        self.apply_filter();
    }

    pub fn update_size(&mut self, index: usize, size: u64, file_count: u64) {
        if let Some(item) = self.results.get_mut(index) {
            item.scan_result.size = Some(size);
            item.scan_result.file_count = Some(file_count);
            self.total_size = self.results.iter().filter_map(|r| r.scan_result.size).sum();
            self.sizes_calculated += 1;
            // Sort immediately so display stays current
            self.sort_results();
            self.apply_filter();
        }
    }

    pub fn on_tick(&mut self) {
        self.spinner_tick = self.spinner_tick.wrapping_add(1);
        self.sort_flash = self.sort_flash.saturating_sub(1);

        // Handle deferred filter (e.g., after deletion)
        if self.needs_filter {
            self.apply_filter();
            self.needs_filter = false;
        }
    }

    fn sort_results(&mut self) {
        match self.sort_order {
            SortOrder::Size => {
                self.results.sort_by(|a, b| {
                    b.scan_result
                        .size
                        .unwrap_or(0)
                        .cmp(&a.scan_result.size.unwrap_or(0))
                });
            }
            SortOrder::Path => {
                self.results
                    .sort_by(|a, b| a.scan_result.path.cmp(&b.scan_result.path));
            }
            SortOrder::Age => {
                self.results.sort_by(|a, b| {
                    let a_time = a.scan_result.modified.unwrap_or(SystemTime::UNIX_EPOCH);
                    let b_time = b.scan_result.modified.unwrap_or(SystemTime::UNIX_EPOCH);
                    a_time.cmp(&b_time)
                });
            }
        }
    }

    fn apply_filter(&mut self) {
        if self.search_query.is_empty() {
            self.filtered_indices = (0..self.results.len()).collect();
        } else {
            let query = self.search_query.to_lowercase();
            self.filtered_indices = self
                .results
                .iter()
                .enumerate()
                .filter(|(_, item)| {
                    !item.is_deleted
                        && item
                            .scan_result
                            .path
                            .to_string_lossy()
                            .to_lowercase()
                            .contains(&query)
                })
                .map(|(i, _)| i)
                .collect();
        }

        // Adjust cursor if out of bounds
        if self.cursor >= self.filtered_indices.len() && !self.filtered_indices.is_empty() {
            self.cursor = self.filtered_indices.len() - 1;
        }
    }

    pub fn visible_results(&self) -> Vec<(usize, &ResultItem)> {
        self.filtered_indices
            .iter()
            .skip(self.scroll_offset)
            .take(self.visible_height)
            .filter_map(|&i| self.results.get(i).map(|item| (i, item)))
            .collect()
    }

    pub fn current_item(&self) -> Option<&ResultItem> {
        self.filtered_indices
            .get(self.cursor)
            .and_then(|&i| self.results.get(i))
    }

    pub fn current_index(&self) -> Option<usize> {
        self.filtered_indices.get(self.cursor).copied()
    }

    pub fn move_cursor(&mut self, delta: isize) {
        let new_pos = (self.cursor as isize + delta)
            .max(0)
            .min(self.filtered_indices.len().saturating_sub(1) as isize)
            as usize;
        self.cursor = new_pos;

        // Adjust scroll offset
        if self.cursor < self.scroll_offset {
            self.scroll_offset = self.cursor;
        } else if self.cursor >= self.scroll_offset + self.visible_height {
            self.scroll_offset = self.cursor - self.visible_height + 1;
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
    }

    pub fn mark_deleting(&mut self, index: usize) {
        if let Some(item) = self.results.get_mut(index) {
            item.is_deleting = true;
        }
    }

    pub fn scan_complete(&mut self) {
        self.scanning = false;
        // Ensure filter is applied when scan finishes
        self.apply_filter();
    }

    pub fn add_error(&mut self, error: String) {
        self.errors.push(error);
    }

    pub fn apply_sort_and_filter(&mut self) {
        self.sort_results();
        self.apply_filter();
        self.sort_flash = 5; // Brief yellow highlight (~500ms)
    }

    pub fn sort_label(&self) -> &'static str {
        match self.sort_order {
            SortOrder::Size => "SIZE",
            SortOrder::Path => "PATH",
            SortOrder::Age => "AGE",
        }
    }

    pub fn is_calculating_sizes(&self) -> bool {
        !self.results.is_empty() && self.sizes_calculated < self.results.len()
    }

    pub fn size_progress(&self) -> f64 {
        if self.results.is_empty() {
            0.0
        } else {
            self.sizes_calculated as f64 / self.results.len() as f64
        }
    }

    pub fn spinner_char(&self) -> char {
        const SPINNER: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
        SPINNER[self.spinner_tick % SPINNER.len()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
