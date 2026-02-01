use bytesize::ByteSize;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};
use std::time::{Duration, SystemTime};

use crate::tui::app::App;

pub fn draw_info(frame: &mut Frame, app: &App, area: Rect) {
    let content = if let Some(item) = app.current_item() {
        let path = item.scan_result.path.to_string_lossy().to_string();

        let size_info = match item.scan_result.size {
            Some(size) => format!("Size: {}", ByteSize::b(size)),
            None => "Size: calculating...".to_string(),
        };

        let age_info = match item.scan_result.modified {
            Some(time) => {
                let age = SystemTime::now()
                    .duration_since(time)
                    .unwrap_or(Duration::ZERO);
                let days = age.as_secs() / 86400;
                format!("Last modified: {} days ago", days)
            }
            None => "Last modified: unknown".to_string(),
        };

        let mut lines = vec![
            Line::from(Span::styled(
                "Path:",
                Style::default().add_modifier(Modifier::BOLD),
            )),
            Line::from(path),
            Line::from(""),
            Line::from(size_info),
            Line::from(age_info),
        ];

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
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "Deleting this may break applications!",
                Style::default().fg(Color::Red),
            )));
        }

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

    let info = Paragraph::new(content)
        .wrap(Wrap { trim: true })
        .block(
            Block::default()
                .title(" Info ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        );

    frame.render_widget(info, area);
}
