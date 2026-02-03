use bytesize::ByteSize;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::tui::app::App;

#[allow(clippy::cast_possible_truncation)] // Truncation acceptable for UI widths
pub fn draw_analytics(frame: &mut Frame, app: &App, area: Rect) {
    // Build all content lines
    let content = build_content(app, area.width as usize);

    // Calculate visible portion based on scroll
    let inner_height = area.height.saturating_sub(2) as usize; // -2 for borders
    let visible: Vec<Line> = content
        .into_iter()
        .skip(app.analytics_scroll)
        .take(inner_height)
        .collect();

    // Render
    let block = Block::default()
        .title(" Analytics ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let paragraph = Paragraph::new(visible).block(block);
    frame.render_widget(paragraph, area);
}

#[allow(clippy::too_many_lines)] // UI rendering benefits from being in one place
#[allow(clippy::cast_precision_loss)] // Precision loss acceptable for UI percentages
#[allow(clippy::cast_possible_truncation)] // Truncation acceptable for bar widths
#[allow(clippy::cast_sign_loss)] // Percentages are always positive
fn build_content(app: &App, width: usize) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();
    let analytics = &app.analytics;

    // === SCAN STATS ===
    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled(
        "  SCAN STATS",
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    )]));
    lines.push(Line::from(format!(
        "  {}",
        "─".repeat(width.saturating_sub(4))
    )));

    let status = if analytics.sizes_complete {
        "Complete"
    } else if analytics.scan_complete {
        "Calculating..."
    } else {
        "Scanning..."
    };
    let elapsed = analytics.elapsed_secs();
    let elapsed_str = if elapsed < 60.0 {
        format!("{elapsed:.1}s")
    } else {
        let mins = elapsed / 60.0;
        format!("{mins:.1}m")
    };
    let rate = format!("{:.1} results/sec", analytics.results_rate());
    lines.push(Line::from(format!(
        "  Status: {:<12} Results: {:<8} Time: {:<10} Rate: {}",
        status,
        analytics.total_count(),
        elapsed_str,
        rate
    )));

    // === TARGET BREAKDOWN ===
    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled(
        "  TARGET BREAKDOWN",
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    )]));
    lines.push(Line::from(format!(
        "  {}",
        "─".repeat(width.saturating_sub(4))
    )));

    let total_count = analytics.total_count();
    lines.push(Line::from(format!(
        "  {:<20} {:>8} {:>6} {:>12}",
        "TYPE", "COUNT", "%", "SIZE"
    )));

    for target in analytics.targets_by_size() {
        let pct = if total_count > 0 {
            (target.count as f32 / total_count as f32) * 100.0
        } else {
            0.0
        };
        lines.push(Line::from(format!(
            "  {:<20} {:>8} {:>5.1}% {:>12}",
            truncate_str(&target.name, 20),
            target.count,
            pct,
            ByteSize::b(target.total_size)
        )));
    }

    // === SIZE DISTRIBUTION (bar chart) ===
    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled(
        "  SIZE DISTRIBUTION",
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    )]));
    lines.push(Line::from(format!(
        "  {}",
        "─".repeat(width.saturating_sub(4))
    )));

    let total_size = analytics.total_size();
    let bar_width = 24;
    for target in analytics.targets_by_size().iter().take(6) {
        let pct = if total_size > 0 {
            target.total_size as f64 / total_size as f64
        } else {
            0.0
        };
        let filled = (pct * bar_width as f64) as usize;
        let bar = format!("{}{}", "█".repeat(filled), "░".repeat(bar_width - filled));
        lines.push(Line::from(format!(
            "  {}  {:<16} {:>12}",
            bar,
            truncate_str(&target.name, 16),
            ByteSize::b(target.total_size)
        )));
    }

    // === PROFILE BREAKDOWN ===
    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled(
        "  PROFILE BREAKDOWN",
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    )]));
    lines.push(Line::from(format!(
        "  {}",
        "─".repeat(width.saturating_sub(4))
    )));

    lines.push(Line::from(format!(
        "  {:<16} {:>8} {:>12} {:>28}",
        "PROFILE", "COUNT", "SIZE", "% OF TOTAL"
    )));

    for profile in analytics.profiles_by_size() {
        let pct = if total_size > 0 {
            (profile.total_size as f64 / total_size as f64) * 100.0
        } else {
            0.0
        };
        let bar_filled = (pct / 100.0 * 24.0) as usize;
        let bar = format!("{}{}", "█".repeat(bar_filled), "░".repeat(24 - bar_filled));
        lines.push(Line::from(format!(
            "  {:<16} {:>8} {:>12}   {} {:>4.1}%",
            profile.name,
            profile.count,
            ByteSize::b(profile.total_size),
            bar,
            pct
        )));
    }

    // === TOP 5 LARGEST ===
    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled(
        "  TOP 5 LARGEST",
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    )]));
    lines.push(Line::from(format!(
        "  {}",
        "─".repeat(width.saturating_sub(4))
    )));

    for item in &analytics.top_largest {
        let path_str = item.path.to_string_lossy();
        let max_path_len = width.saturating_sub(20);
        let display_path = if path_str.len() > max_path_len {
            format!("...{}", &path_str[path_str.len() - max_path_len + 3..])
        } else {
            path_str.to_string()
        };
        lines.push(Line::from(format!(
            "  {:<width$} {:>12}",
            display_path,
            ByteSize::b(item.size),
            width = max_path_len
        )));
    }

    // Padding at bottom for scroll
    lines.push(Line::from(""));
    lines.push(Line::from(""));

    lines
}

/// Calculate total number of content lines (for scroll bounds)
pub fn content_line_count(app: &App) -> usize {
    // stats(4) + targets(N+4) + distribution(6+4) + profiles(N+4) + largest(5+4) + padding(2)
    let target_count = app.analytics.by_target.len();
    let profile_count = app.analytics.by_profile.len();
    4 + (target_count + 4) + (6 + 4) + (profile_count + 4) + (5 + 4) + 2
}

fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::app::SortOrder;
    use std::path::PathBuf;

    #[test]
    fn test_truncate_str_short_string() {
        assert_eq!(truncate_str("hello", 10), "hello");
    }

    #[test]
    fn test_truncate_str_exact_length() {
        assert_eq!(truncate_str("hello", 5), "hello");
    }

    #[test]
    fn test_truncate_str_long_string() {
        assert_eq!(truncate_str("hello world", 8), "hello...");
    }

    #[test]
    fn test_truncate_str_very_short_max() {
        assert_eq!(truncate_str("hello", 3), "...");
    }

    #[test]
    fn test_content_line_count_empty() {
        let app = App::new(false, SortOrder::Size, false);
        // 4 + (0+4) + (6+4) + (0+4) + (5+4) + 2 = 33
        assert_eq!(content_line_count(&app), 33);
    }

    #[test]
    fn test_content_line_count_with_data() {
        let mut app = App::new(false, SortOrder::Size, false);

        // Add 3 different targets (2 profiles: node, python)
        app.analytics
            .record_result(&PathBuf::from("/a/node_modules"), Some(100));
        app.analytics
            .record_result(&PathBuf::from("/b/.next"), Some(200));
        app.analytics
            .record_result(&PathBuf::from("/c/.venv"), Some(300));

        // 4 + (3+4) + (6+4) + (2+4) + (5+4) + 2 = 38
        assert_eq!(content_line_count(&app), 38);
    }
}
