use bytesize::ByteSize;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};
use std::path::Path;
use std::time::{Duration, SystemTime};

use crate::tui::app::App;

const MB: u64 = 1024 * 1024;
const SIZE_SMALL: u64 = 100 * MB; // ≤100MB = green
const SIZE_MEDIUM: u64 = 600 * MB; // 100-600MB = yellow, >600MB = red

const AGE_RECENT: u64 = 30; // <30 days = red
const AGE_MEDIUM: u64 = 90; // 30-90 days = yellow, >90 days = green

pub fn draw_info(frame: &mut Frame, app: &App, area: Rect) {
    let content = if let Some(item) = app.current_item() {
        let path = &item.scan_result.path;
        let mut lines = Vec::new();

        // Path with colored target folder
        let (parent, target) = split_path(path);
        lines.push(Line::from(vec![
            Span::raw(parent),
            Span::styled(
                target,
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::from(""));

        // Type
        let target_type = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());
        lines.push(Line::from(vec![
            Span::styled("Type:      ", Style::default().fg(Color::DarkGray)),
            Span::raw(target_type),
        ]));

        // Size with color and bar
        if let Some(size) = item.scan_result.size {
            let size_color = size_to_color(size);
            let size_label = size_to_label(size);
            let bar = size_bar(size, 10);
            lines.push(Line::from(vec![
                Span::styled("Size:      ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    format!("{:<10}", ByteSize::b(size)),
                    Style::default().fg(size_color),
                ),
                Span::styled(bar, Style::default().fg(size_color)),
                Span::styled(
                    format!(" ({})", size_label),
                    Style::default().fg(Color::DarkGray),
                ),
            ]));
        } else {
            lines.push(Line::from(vec![
                Span::styled("Size:      ", Style::default().fg(Color::DarkGray)),
                Span::styled("calculating...", Style::default().fg(Color::DarkGray)),
            ]));
        }

        // Age with color
        if let Some(time) = item.scan_result.modified {
            let age = SystemTime::now()
                .duration_since(time)
                .unwrap_or(Duration::ZERO);
            let days = age.as_secs() / 86400;
            let age_color = age_to_color(days);
            let age_label = age_to_label(days);
            lines.push(Line::from(vec![
                Span::styled("Age:       ", Style::default().fg(Color::DarkGray)),
                Span::styled(format!("{} days", days), Style::default().fg(age_color)),
                Span::styled(
                    format!(" ({})", age_label),
                    Style::default().fg(Color::DarkGray),
                ),
            ]));
        } else {
            lines.push(Line::from(vec![
                Span::styled("Age:       ", Style::default().fg(Color::DarkGray)),
                Span::raw("unknown"),
            ]));
        }

        // File count
        if let Some(count) = item.scan_result.file_count {
            lines.push(Line::from(vec![
                Span::styled("Files:     ", Style::default().fg(Color::DarkGray)),
                Span::raw(format_file_count(count)),
            ]));
        }

        // Disk usage %
        if let Some(size) = item.scan_result.size {
            if app.total_size > 0 {
                let pct = (size as f64 / app.total_size as f64) * 100.0;
                lines.push(Line::from(vec![
                    Span::styled("Share:     ", Style::default().fg(Color::DarkGray)),
                    Span::raw(format!("{:.1}% of scanned", pct)),
                ]));
            }
        }

        // Project name
        if let Some(project) = find_project_name(path) {
            lines.push(Line::from(vec![
                Span::styled("Project:   ", Style::default().fg(Color::DarkGray)),
                Span::raw(project),
            ]));
        }

        // Sensitive warning
        if item.risk.is_sensitive {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "⚠️ SENSITIVE DIRECTORY",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )));
            if let Some(ref reason) = item.risk.reason {
                lines.push(Line::from(Span::styled(
                    reason.clone(),
                    Style::default().fg(Color::Yellow),
                )));
            }
            lines.push(Line::from(Span::styled(
                "Deleting this may break applications!",
                Style::default().fg(Color::Red),
            )));
        }

        // Deleted status
        if item.is_deleted {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "✓ DELETED",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )));
        }

        lines
    } else {
        vec![Line::from("No item selected")]
    };

    let info = Paragraph::new(content).wrap(Wrap { trim: true }).block(
        Block::default()
            .title(" Info ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    );

    frame.render_widget(info, area);
}

fn split_path(path: &Path) -> (String, String) {
    let full = path.to_string_lossy();
    if let Some(name) = path.file_name() {
        let name_str = name.to_string_lossy();
        let parent = full.strip_suffix(&*name_str).unwrap_or(&full);
        (parent.to_string(), name_str.to_string())
    } else {
        (full.to_string(), String::new())
    }
}

fn size_to_color(size: u64) -> Color {
    if size <= SIZE_SMALL {
        Color::Green
    } else if size <= SIZE_MEDIUM {
        Color::Yellow
    } else {
        Color::Red
    }
}

fn size_to_label(size: u64) -> &'static str {
    if size <= SIZE_SMALL {
        "small"
    } else if size <= SIZE_MEDIUM {
        "medium"
    } else {
        "large"
    }
}

fn size_bar(size: u64, width: usize) -> String {
    // Cap at 1GB for bar calculation
    let max_size = 1024 * MB;
    let ratio = (size as f64 / max_size as f64).min(1.0);
    let filled = (ratio * width as f64).round() as usize;
    let empty = width.saturating_sub(filled);
    format!(" {}{}", "█".repeat(filled), "░".repeat(empty))
}

fn age_to_color(days: u64) -> Color {
    if days < AGE_RECENT {
        Color::Red // Recent = risky to delete
    } else if days < AGE_MEDIUM {
        Color::Yellow
    } else {
        Color::Green // Old = safe to delete
    }
}

fn age_to_label(days: u64) -> &'static str {
    if days < AGE_RECENT {
        "recent"
    } else if days < AGE_MEDIUM {
        "moderate"
    } else {
        "stale"
    }
}

fn format_file_count(count: u64) -> String {
    if count >= 1_000_000 {
        format!("{:.1}M files", count as f64 / 1_000_000.0)
    } else if count >= 1_000 {
        format!("{:.1}K files", count as f64 / 1_000.0)
    } else {
        format!("{} files", count)
    }
}

fn find_project_name(target_path: &Path) -> Option<String> {
    // Go up to parent directory (e.g., from node_modules to project root)
    let parent = target_path.parent()?;

    // Check for package.json (Node.js)
    let pkg_json = parent.join("package.json");
    if pkg_json.exists() {
        if let Ok(content) = std::fs::read_to_string(&pkg_json) {
            if let Some(name) = extract_json_field(&content, "name") {
                return Some(name);
            }
        }
    }

    // Check for Cargo.toml (Rust)
    let cargo_toml = parent.join("Cargo.toml");
    if cargo_toml.exists() {
        if let Ok(content) = std::fs::read_to_string(&cargo_toml) {
            if let Some(name) = extract_toml_name(&content) {
                return Some(name);
            }
        }
    }

    // Check for pyproject.toml (Python)
    let pyproject = parent.join("pyproject.toml");
    if pyproject.exists() {
        if let Ok(content) = std::fs::read_to_string(&pyproject) {
            if let Some(name) = extract_toml_name(&content) {
                return Some(name);
            }
        }
    }

    // Fallback: use parent directory name
    parent.file_name().map(|n| n.to_string_lossy().to_string())
}

fn extract_json_field(content: &str, field: &str) -> Option<String> {
    // Simple extraction without full JSON parsing
    let pattern = format!("\"{}\"", field);
    let idx = content.find(&pattern)?;
    let rest = &content[idx + pattern.len()..];
    let colon = rest.find(':')?;
    let after_colon = rest[colon + 1..].trim_start();
    if after_colon.starts_with('"') {
        let value_start = 1;
        let value_end = after_colon[1..].find('"')?;
        return Some(after_colon[value_start..value_start + value_end].to_string());
    }
    None
}

fn extract_toml_name(content: &str) -> Option<String> {
    // Simple extraction: look for name = "value" under [package] or [project]
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("name") && line.contains('=') {
            let parts: Vec<&str> = line.splitn(2, '=').collect();
            if parts.len() == 2 {
                let value = parts[1].trim().trim_matches('"').trim_matches('\'');
                if !value.is_empty() {
                    return Some(value.to_string());
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    // split_path tests
    #[test]
    fn test_split_path_basic() {
        let path = Path::new("/home/user/project/node_modules");
        let (parent, target) = split_path(path);
        assert_eq!(parent, "/home/user/project/");
        assert_eq!(target, "node_modules");
    }

    #[test]
    fn test_split_path_root() {
        let path = Path::new("/node_modules");
        let (parent, target) = split_path(path);
        assert_eq!(parent, "/");
        assert_eq!(target, "node_modules");
    }

    // size_to_color tests
    #[test]
    fn test_size_to_color_small() {
        assert_eq!(size_to_color(50 * MB), Color::Green);
        assert_eq!(size_to_color(100 * MB), Color::Green);
    }

    #[test]
    fn test_size_to_color_medium() {
        assert_eq!(size_to_color(101 * MB), Color::Yellow);
        assert_eq!(size_to_color(600 * MB), Color::Yellow);
    }

    #[test]
    fn test_size_to_color_large() {
        assert_eq!(size_to_color(601 * MB), Color::Red);
        assert_eq!(size_to_color(2000 * MB), Color::Red);
    }

    // size_to_label tests
    #[test]
    fn test_size_to_label() {
        assert_eq!(size_to_label(50 * MB), "small");
        assert_eq!(size_to_label(100 * MB), "small");
        assert_eq!(size_to_label(101 * MB), "medium");
        assert_eq!(size_to_label(600 * MB), "medium");
        assert_eq!(size_to_label(601 * MB), "large");
    }

    // age_to_color tests
    #[test]
    fn test_age_to_color_recent() {
        assert_eq!(age_to_color(0), Color::Red);
        assert_eq!(age_to_color(29), Color::Red);
    }

    #[test]
    fn test_age_to_color_moderate() {
        assert_eq!(age_to_color(30), Color::Yellow);
        assert_eq!(age_to_color(89), Color::Yellow);
    }

    #[test]
    fn test_age_to_color_stale() {
        assert_eq!(age_to_color(90), Color::Green);
        assert_eq!(age_to_color(365), Color::Green);
    }

    // age_to_label tests
    #[test]
    fn test_age_to_label() {
        assert_eq!(age_to_label(0), "recent");
        assert_eq!(age_to_label(29), "recent");
        assert_eq!(age_to_label(30), "moderate");
        assert_eq!(age_to_label(89), "moderate");
        assert_eq!(age_to_label(90), "stale");
        assert_eq!(age_to_label(365), "stale");
    }

    // format_file_count tests
    #[test]
    fn test_format_file_count_small() {
        assert_eq!(format_file_count(1), "1 files");
        assert_eq!(format_file_count(999), "999 files");
    }

    #[test]
    fn test_format_file_count_thousands() {
        assert_eq!(format_file_count(1000), "1.0K files");
        assert_eq!(format_file_count(1500), "1.5K files");
        assert_eq!(format_file_count(999_999), "1000.0K files");
    }

    #[test]
    fn test_format_file_count_millions() {
        assert_eq!(format_file_count(1_000_000), "1.0M files");
        assert_eq!(format_file_count(2_500_000), "2.5M files");
    }

    // size_bar tests
    #[test]
    fn test_size_bar_empty() {
        let bar = size_bar(0, 10);
        assert_eq!(bar, " ░░░░░░░░░░");
    }

    #[test]
    fn test_size_bar_full() {
        let bar = size_bar(1024 * MB, 10); // 1GB = max
        assert_eq!(bar, " ██████████");
    }

    #[test]
    fn test_size_bar_half() {
        let bar = size_bar(512 * MB, 10); // ~half of 1GB
        assert!(bar.contains("█") && bar.contains("░"));
    }

    // extract_json_field tests
    #[test]
    fn test_extract_json_field_basic() {
        let json = r#"{"name": "my-project", "version": "1.0.0"}"#;
        assert_eq!(
            extract_json_field(json, "name"),
            Some("my-project".to_string())
        );
    }

    #[test]
    fn test_extract_json_field_with_whitespace() {
        let json = r#"{
            "name"  :  "spaced-project",
            "version": "1.0.0"
        }"#;
        assert_eq!(
            extract_json_field(json, "name"),
            Some("spaced-project".to_string())
        );
    }

    #[test]
    fn test_extract_json_field_missing() {
        let json = r#"{"version": "1.0.0"}"#;
        assert_eq!(extract_json_field(json, "name"), None);
    }

    // extract_toml_name tests
    #[test]
    fn test_extract_toml_name_cargo() {
        let toml = r#"
[package]
name = "my-rust-project"
version = "0.1.0"
"#;
        assert_eq!(extract_toml_name(toml), Some("my-rust-project".to_string()));
    }

    #[test]
    fn test_extract_toml_name_pyproject() {
        let toml = r#"
[project]
name = "my-python-project"
version = "1.0.0"
"#;
        assert_eq!(
            extract_toml_name(toml),
            Some("my-python-project".to_string())
        );
    }

    #[test]
    fn test_extract_toml_name_single_quotes() {
        let toml = "name = 'single-quoted'";
        assert_eq!(extract_toml_name(toml), Some("single-quoted".to_string()));
    }

    #[test]
    fn test_extract_toml_name_missing() {
        let toml = "[package]\nversion = \"1.0.0\"";
        assert_eq!(extract_toml_name(toml), None);
    }
}
