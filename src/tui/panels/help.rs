use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub fn draw_help(frame: &mut Frame, area: Rect) {
    let content = vec![
        Line::from(Span::styled(
            "Navigation",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from("  ↑/k      Move up"),
        Line::from("  ↓/j      Move down"),
        Line::from("  PgUp/u   Page up"),
        Line::from("  PgDn/d   Page down"),
        Line::from("  Home     First result"),
        Line::from("  End      Last result"),
        Line::from("  ←/h      Options panel"),
        Line::from("  →/l      Info panel"),
        Line::from(""),
        Line::from(Span::styled(
            "Actions",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from("  SPACE    Delete folder"),
        Line::from("  /        Search mode"),
        Line::from("  t        Multi-select"),
        Line::from("  s        Cycle sort"),
        Line::from("  e        Show errors"),
        Line::from("  q/Esc    Quit"),
        Line::from(""),
        Line::from(Span::styled(
            "Multi-Select Mode",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from("  SPACE    Toggle selection"),
        Line::from("  a        Select/deselect all"),
        Line::from("  Enter    Delete selected"),
        Line::from("  t/Esc    Exit mode"),
    ];

    let help = Paragraph::new(content).block(
        Block::default()
            .title(" Help ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    );

    frame.render_widget(help, area);
}
