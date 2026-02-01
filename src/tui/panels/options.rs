use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::tui::app::{App, SortOrder};

pub fn draw_options(frame: &mut Frame, app: &App, area: Rect) {
    let sort_indicator = match app.sort_order {
        SortOrder::Size => "[x] Size  [ ] Path  [ ] Age",
        SortOrder::Path => "[ ] Size  [x] Path  [ ] Age",
        SortOrder::Age => "[ ] Size  [ ] Path  [x] Age",
    };

    let content = vec![
        Line::from(Span::styled(
            "Sort Order (press 's' to cycle):",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(sort_indicator),
        Line::from(""),
        Line::from(Span::styled(
            "Exclude Sensitive:",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(if app.exclude_sensitive {
            "[x] Enabled"
        } else {
            "[ ] Disabled"
        }),
        Line::from(""),
        Line::from(Span::styled(
            "Statistics:",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(format!("Total results: {}", app.results.len())),
        Line::from(format!("Filtered: {}", app.filtered_indices.len())),
        Line::from(format!("Errors: {}", app.errors.len())),
    ];

    let options = Paragraph::new(content).block(
        Block::default()
            .title(" Options ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    );

    frame.render_widget(options, area);
}
