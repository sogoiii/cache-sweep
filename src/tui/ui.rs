use std::time::{Duration, SystemTime};

use bytesize::ByteSize;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph},
    Frame,
};

use super::app::{App, Mode, Panel};
use super::panels;

pub fn draw(frame: &mut Frame, app: &App) {
    let show_progress = app.scanning || app.is_calculating_sizes();
    let header_height = if show_progress { 4 } else { 3 };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(header_height), // Header (expands when scanning)
            Constraint::Min(10),               // Main content
            Constraint::Length(3),             // Footer/Status
        ])
        .split(frame.area());

    draw_header(frame, app, chunks[0], show_progress);
    draw_main(frame, app, chunks[1]);
    draw_footer(frame, app, chunks[2]);
}

fn draw_header(frame: &mut Frame, app: &App, area: Rect, show_progress: bool) {
    let sort_label = app.sort_label();

    let prefix = format!(
        " cache-sweep | {} results | {} potential | {} freed | sort:",
        app.filtered_indices.len(),
        ByteSize::b(app.total_size),
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
        // Split header into title + progress bar
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(2), Constraint::Length(2)])
            .split(area);

        // Title line
        let header = Paragraph::new(title_line)
            .block(Block::default().borders(Borders::TOP | Borders::LEFT | Borders::RIGHT));
        frame.render_widget(header, chunks[0]);

        // Progress line
        let progress_label = if app.scanning {
            format!(" {} Scanning... ", app.spinner_char())
        } else {
            format!(
                " {} Calculating sizes {}/{} ",
                app.spinner_char(),
                app.sizes_calculated,
                app.results.len()
            )
        };

        let gauge = Gauge::default()
            .block(Block::default().borders(Borders::BOTTOM | Borders::LEFT | Borders::RIGHT))
            .gauge_style(Style::default().fg(Color::Green))
            .ratio(app.size_progress())
            .label(progress_label);
        frame.render_widget(gauge, chunks[1]);
    } else {
        let header = Paragraph::new(title_line).block(Block::default().borders(Borders::ALL));
        frame.render_widget(header, area);
    }
}

fn draw_main(frame: &mut Frame, app: &App, area: Rect) {
    match app.panel {
        Panel::Results => draw_results_panel(frame, app, area),
        Panel::Info => panels::draw_info(frame, app, area),
        Panel::Options => panels::draw_options(frame, app, area),
        Panel::Help => panels::draw_help(frame, area),
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
        Mode::MultiSelect => format!(" Results ({} selected) ", app.selected_indices.len()),
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
    let visible = app.visible_results();
    let mut items: Vec<ListItem> = Vec::with_capacity(visible.len());
    items.extend(
        visible
            .iter()
            .enumerate()
            .map(|(display_idx, (_real_idx, item))| {
                let is_cursor = display_idx + app.scroll_offset == app.cursor;
                let path = item.scan_result.path.to_string_lossy();

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

                let style = if item.is_deleted {
                    Style::default().fg(Color::DarkGray)
                } else if is_cursor {
                    Style::default()
                        .bg(Color::Blue)
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD)
                } else if item.is_selected {
                    Style::default().fg(Color::LightBlue)
                } else if item.risk.is_sensitive {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default()
                };

                ListItem::new(Line::from(Span::styled(line_content, style)))
            }),
    );

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::BOTTOM | Borders::LEFT | Borders::RIGHT)
            .border_style(Style::default().fg(Color::Cyan)),
    );

    frame.render_widget(list, chunks[1]);
}

fn draw_footer(frame: &mut Frame, app: &App, area: Rect) {
    let help_text = match app.mode {
        Mode::Normal => match app.panel {
            Panel::Info => "↑/↓:navigate | ←:back | o:open | q:quit",
            Panel::Analytics => "↑/↓:scroll | a/Esc:back | q:quit",
            _ => "↑/↓:navigate | SPACE:delete | /:search | t:multi-select | s:sort | a:analytics | q:quit",
        },
        Mode::Search => "Type to filter | Enter:confirm | Esc:cancel",
        Mode::MultiSelect => "SPACE:toggle | a:all | Enter:delete selected | t/Esc:exit",
    };

    let footer = Paragraph::new(help_text)
        .style(Style::default().fg(Color::DarkGray))
        .block(Block::default().borders(Borders::ALL));

    frame.render_widget(footer, area);
}
