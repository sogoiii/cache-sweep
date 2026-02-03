use std::time::{Duration, SystemTime};

use bytesize::ByteSize;
use ratatui::{
    layout::{Constraint, Direction, Flex, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

use super::app::{App, Mode, Panel};
use super::panels;
use super::widgets::DualProgressBar;

/// Determines the style for a result item based on its state.
#[allow(clippy::fn_params_excessive_bools)] // Bools map directly to item state flags
fn result_item_style(
    is_cursor: bool,
    is_deleted: bool,
    is_selected: bool,
    is_sensitive: bool,
) -> Style {
    if is_cursor {
        if is_deleted {
            // Cursor on deleted: dim background, still visible
            Style::default()
                .bg(Color::DarkGray)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .bg(Color::Blue)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD)
        }
    } else if is_deleted {
        Style::default().fg(Color::DarkGray)
    } else if is_selected {
        Style::default().fg(Color::LightBlue)
    } else if is_sensitive {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    }
}

pub fn draw(frame: &mut Frame, app: &mut App) {
    let show_progress = app.scanning || app.is_calculating_sizes();
    // Gradient bar needs: 1 (title border top) + 1 (title text) + 1 (bar) + 1 (labels) + 1 (border bottom) = 5
    // Or when not showing progress: 3 (standard header with borders)
    let header_height = if show_progress { 5 } else { 3 };

    // Show tabs row when there are multiple target types
    let show_tabs = app.target_groups.len() > 1;
    let tabs_height = u16::from(show_tabs);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(header_height), // Header (expands when scanning)
            Constraint::Length(tabs_height),   // Tabs row (if multiple targets)
            Constraint::Min(10),               // Main content
            Constraint::Length(3),             // Footer/Status
        ])
        .split(frame.area());

    draw_header(frame, app, chunks[0], show_progress);
    if show_tabs {
        draw_tabs(frame, app, chunks[1]);
    }
    draw_main(frame, app, chunks[2]);
    draw_footer(frame, app, chunks[3]);

    if app.mode == Mode::Confirm {
        draw_confirm_popup(frame, app);
    }
}

fn draw_header(frame: &mut Frame, app: &App, area: Rect, show_progress: bool) {
    let sort_label = app.sort_label();

    let prefix = format!(
        " cache-sweep | {} results | {} potential | {} freed | sort:",
        app.filtered_indices.len(),
        ByteSize::b(app.active_tab_subtotal()),
        ByteSize::b(app.freed_size),
    );

    let base_style = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);
    let sort_style = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);

    let title_line = Line::from(vec![
        Span::styled(prefix, base_style),
        Span::styled(sort_label, sort_style),
        Span::styled(" ", base_style),
    ]);

    if show_progress {
        // Split header into: title (2 lines) + gradient bar (2 lines) + bottom border (1 line)
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2), // Title with top/side borders
                Constraint::Length(2), // Gradient bar + labels
                Constraint::Length(1), // Bottom border
            ])
            .split(area);

        // Title line
        let header = Paragraph::new(title_line)
            .block(Block::default().borders(Borders::TOP | Borders::LEFT | Borders::RIGHT));
        frame.render_widget(header, chunks[0]);

        // Gradient progress bar with labels
        // Inner width accounting for side borders
        let inner_area = Rect {
            x: chunks[1].x + 1,
            y: chunks[1].y,
            width: chunks[1].width.saturating_sub(2),
            height: chunks[1].height,
        };

        let dual_bar = DualProgressBar::new()
            .scan_complete(!app.scanning)
            .scan_count(app.results.len())
            .size_progress(app.sizes_calculated, app.results.len())
            .spinner_frame(app.spinner_tick);

        frame.render_widget(dual_bar, inner_area);

        // Side borders for bar area
        let bar_border = Block::default().borders(Borders::LEFT | Borders::RIGHT);
        frame.render_widget(bar_border, chunks[1]);

        // Bottom border
        let bottom_border =
            Block::default().borders(Borders::BOTTOM | Borders::LEFT | Borders::RIGHT);
        frame.render_widget(bottom_border, chunks[2]);
    } else {
        let header = Paragraph::new(title_line).block(Block::default().borders(Borders::ALL));
        frame.render_widget(header, area);
    }
}

fn draw_tabs(frame: &mut Frame, app: &mut App, area: Rect) {
    let all_count = app.results.iter().filter(|r| !r.is_deleted).count();
    let all_tab = format!("All ({all_count})");
    let all_tab_width = all_tab.len() + 3; // + padding/divider

    // Calculate available width for scrollable groups
    let available_width = area.width as usize;
    let indicator_width = 4; // " ◀ " or " ▶ "
    let scrollable_width = available_width.saturating_sub(all_tab_width + indicator_width * 2);

    // Build group tab strings and calculate which ones fit
    let group_tabs: Vec<String> = app
        .target_groups
        .iter()
        .map(|g| format!("{} ({})", g.name, g.count))
        .collect();

    // Calculate how many groups fit starting from scroll offset
    let mut visible_groups = Vec::new();
    let mut used_width = 0;
    let divider_width = 3; // " │ "

    for (i, tab) in group_tabs.iter().enumerate().skip(app.tab_scroll_offset) {
        let tab_width = tab.len() + divider_width;
        if used_width + tab_width > scrollable_width && !visible_groups.is_empty() {
            break;
        }
        visible_groups.push((i, tab.clone()));
        used_width += tab_width;
    }

    // Update app's visible count for scroll calculations
    app.visible_group_count = visible_groups.len();

    // Check for overflow indicators
    let has_left_overflow = app.tab_scroll_offset > 0;
    let has_right_overflow = app.tab_scroll_offset + visible_groups.len() < app.target_groups.len();

    // Build the spans
    let mut spans: Vec<Span> = Vec::new();

    // "All" tab (always visible, highlighted if selected)
    let all_style = if app.active_tab == 0 {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    spans.push(Span::styled(all_tab, all_style));
    spans.push(Span::styled(" │ ", Style::default().fg(Color::DarkGray)));

    // Left overflow indicator
    if has_left_overflow {
        spans.push(Span::styled("◀ ", Style::default().fg(Color::Yellow)));
    }

    // Visible group tabs
    for (i, (group_idx, tab_text)) in visible_groups.iter().enumerate() {
        let is_selected = app.active_tab == group_idx + 1; // +1 because 0 is "All"
        let style = if is_selected {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        spans.push(Span::styled(tab_text.clone(), style));

        // Add divider between groups (not after last)
        if i < visible_groups.len() - 1 {
            spans.push(Span::styled(" │ ", Style::default().fg(Color::DarkGray)));
        }
    }

    // Right overflow indicator
    if has_right_overflow {
        spans.push(Span::styled(" ▶", Style::default().fg(Color::Yellow)));
    }

    let line = Line::from(spans);
    let paragraph = Paragraph::new(line);
    frame.render_widget(paragraph, area);
}

fn draw_main(frame: &mut Frame, app: &App, area: Rect) {
    match app.panel {
        Panel::Results => draw_results_panel(frame, app, area),
        Panel::Info => panels::draw_info(frame, app, area),
        Panel::Analytics => panels::draw_analytics(frame, app, area),
    }
}

#[allow(clippy::too_many_lines)] // UI rendering benefits from being in one place
fn draw_results_panel(frame: &mut Frame, app: &App, area: Rect) {
    // Split area: header row + list
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // Title + header row (no gap)
            Constraint::Min(1),    // Results list
        ])
        .split(area);

    let title = match app.mode {
        Mode::Search => format!(" Results (search: {}_) ", app.search_query),
        Mode::MultiSelect | Mode::Confirm => {
            format!(" Results ({} selected) ", app.selected_indices.len())
        }
        Mode::Normal => " Results - SPACE to delete ".to_string(),
    };

    // Calculate column positions based on area width
    let inner_width = area.width.saturating_sub(2) as usize; // -2 for borders
    let age_width = 8; // "Last_mod" column
    let size_width = 12; // "Size" column
    let path_width = inner_width.saturating_sub(age_width + size_width + 2);

    // Header with column labels
    let header_line = format!(
        "{:<path_width$} {:>age_width$} {:>size_width$}",
        "Path",
        "Last_mod",
        "Size",
        path_width = path_width,
        age_width = age_width,
        size_width = size_width
    );

    let header = Paragraph::new(Line::from(vec![Span::styled(
        header_line,
        Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::BOLD),
    )]))
    .block(
        Block::default()
            .title(title)
            .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT)
            .border_style(Style::default().fg(Color::Cyan)),
    );
    frame.render_widget(header, chunks[0]);

    // Results list
    let items: Vec<ListItem> = build_list_items(app, age_width, size_width, path_width);

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::BOTTOM | Borders::LEFT | Borders::RIGHT)
            .border_style(Style::default().fg(Color::Cyan)),
    );

    frame.render_widget(list, chunks[1]);
}

fn popup_area(area: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let vertical = Layout::vertical([Constraint::Percentage(percent_y)]).flex(Flex::Center);
    let horizontal = Layout::horizontal([Constraint::Percentage(percent_x)]).flex(Flex::Center);
    let [area] = vertical.areas(area);
    let [area] = horizontal.areas(area);
    area
}

fn draw_confirm_popup(frame: &mut Frame, app: &App) {
    let area = popup_area(frame.area(), 35, 15);
    frame.render_widget(Clear, area);

    let total_size: u64 = app
        .selected_indices
        .iter()
        .filter_map(|&idx| app.results.get(idx))
        .filter_map(|r| r.scan_result.size)
        .sum();

    let text = format!(
        "Permanently delete {} items ({})?\n\n[Y]es  /  [N]o",
        app.selected_indices.len(),
        ByteSize::b(total_size)
    );

    let block = Block::bordered().title(" Confirm ");
    let paragraph = Paragraph::new(text).block(block).centered();
    frame.render_widget(paragraph, area);
}

/// Builds list items for the results view
fn build_list_items(
    app: &App,
    age_width: usize,
    size_width: usize,
    path_width: usize,
) -> Vec<ListItem<'static>> {
    let visible = app.visible_results();
    visible
        .iter()
        .enumerate()
        .map(|(display_idx, (_real_idx, item))| {
            let is_cursor = display_idx + app.scroll_offset == app.cursor;
            build_result_item(app, item, is_cursor, age_width, size_width, path_width)
        })
        .collect()
}

/// Builds a single result item row
fn build_result_item(
    app: &App,
    item: &super::app::ResultItem,
    is_cursor: bool,
    age_width: usize,
    size_width: usize,
    path_width: usize,
) -> ListItem<'static> {
    let path = item.scan_result.path.to_string_lossy().to_string();

    // Size display
    let size_str = item.scan_result.size.map_or_else(
        || format!("{:>width$}", "...", width = size_width),
        |size| format!("{:>width$}", ByteSize::b(size), width = size_width),
    );

    // Age display
    let age_str = item.scan_result.modified.map_or_else(
        || format!("{:>width$}", "?", width = age_width),
        |time| {
            let age = SystemTime::now()
                .duration_since(time)
                .unwrap_or(Duration::ZERO);
            let days = age.as_secs() / 86400;
            format!("{:>width$}", format!("{days}d"), width = age_width)
        },
    );

    // Status indicators
    let status = if item.is_deleted {
        "[DELETED] "
    } else if item.is_deleting {
        "[DELETING] "
    } else if item.risk.is_sensitive {
        "⚠️ "
    } else {
        ""
    };

    let selection_marker = if item.is_selected { "[x] " } else { "[ ] " };

    // Build the path portion with status
    let path_portion = if app.mode == Mode::MultiSelect {
        format!("{selection_marker}{status}{path}")
    } else {
        format!("{status}{path}")
    };

    let line_content = format!("{path_portion:<path_width$} {age_str} {size_str}");

    let style = result_item_style(
        is_cursor,
        item.is_deleted,
        item.is_selected,
        item.risk.is_sensitive,
    );

    ListItem::new(Line::from(Span::styled(line_content, style)))
}

fn draw_footer(frame: &mut Frame, app: &App, area: Rect) {
    let has_tabs = app.target_groups.len() > 1;

    let help_text = match app.mode {
        Mode::Normal => match app.panel {
            Panel::Info => "↑/↓:navigate | ←:back | o:open | q:quit".to_string(),
            Panel::Analytics => "↑/↓:scroll | a/Esc:back | q:quit".to_string(),
            _ if has_tabs => "Tab/⇧Tab:switch | ↑/↓:nav | /:search | s:sort | t:multi | SPACE:del | a:stats | q:quit".to_string(),
            Panel::Results => "↑/↓:nav | /:search | s:sort | t:multi | SPACE:del | a:stats | q:quit".to_string(),
        },
        Mode::Search => "Type to filter | Enter:confirm | Esc:cancel".to_string(),
        Mode::MultiSelect => "SPACE:toggle | a:all | Enter:delete selected | t/Esc:exit".to_string(),
        Mode::Confirm => "Y:confirm | N/Esc:cancel".to_string(),
    };

    let footer = Paragraph::new(help_text)
        .style(Style::default().fg(Color::DarkGray))
        .block(Block::default().borders(Borders::ALL));

    frame.render_widget(footer, area);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cursor_on_normal_item() {
        let style = result_item_style(true, false, false, false);
        assert_eq!(style.bg, Some(Color::Blue));
        assert_eq!(style.fg, Some(Color::White));
    }

    #[test]
    fn test_cursor_on_deleted_item() {
        let style = result_item_style(true, true, false, false);
        // Cursor still visible on deleted items with dim background
        assert_eq!(style.bg, Some(Color::DarkGray));
        assert_eq!(style.fg, Some(Color::White));
    }

    #[test]
    fn test_deleted_item_no_cursor() {
        let style = result_item_style(false, true, false, false);
        assert_eq!(style.bg, None);
        assert_eq!(style.fg, Some(Color::DarkGray));
    }

    #[test]
    fn test_selected_item_no_cursor() {
        let style = result_item_style(false, false, true, false);
        assert_eq!(style.fg, Some(Color::LightBlue));
    }

    #[test]
    fn test_sensitive_item_no_cursor() {
        let style = result_item_style(false, false, false, true);
        assert_eq!(style.fg, Some(Color::Yellow));
    }

    #[test]
    fn test_normal_item_no_cursor() {
        let style = result_item_style(false, false, false, false);
        assert_eq!(style, Style::default());
    }

    #[test]
    fn test_cursor_takes_priority_over_selected() {
        let style = result_item_style(true, false, true, false);
        // Cursor style wins over selected
        assert_eq!(style.bg, Some(Color::Blue));
    }

    #[test]
    fn test_cursor_takes_priority_over_sensitive() {
        let style = result_item_style(true, false, false, true);
        // Cursor style wins over sensitive
        assert_eq!(style.bg, Some(Color::Blue));
    }

    #[test]
    fn test_deleted_takes_priority_over_selected() {
        let style = result_item_style(false, true, true, false);
        // Deleted style wins over selected (no cursor)
        assert_eq!(style.fg, Some(Color::DarkGray));
    }

    // === Popup area tests ===

    #[test]
    fn test_popup_area_centers_correctly() {
        let area = Rect::new(0, 0, 100, 100);
        let popup = popup_area(area, 50, 50);

        // 50% of 100 = 50, centered means starting at 25
        assert_eq!(popup.width, 50);
        assert_eq!(popup.height, 50);
        assert_eq!(popup.x, 25);
        assert_eq!(popup.y, 25);
    }

    #[test]
    fn test_popup_area_small_percentages() {
        let area = Rect::new(0, 0, 100, 50);
        let popup = popup_area(area, 20, 20);

        assert_eq!(popup.width, 20);
        assert_eq!(popup.height, 10); // 20% of 50
    }
}
