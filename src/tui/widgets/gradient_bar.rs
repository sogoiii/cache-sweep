use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Widget,
};

/// Two side-by-side progress bars: scan (left) + sizing (right).
///
/// Visual representation:
/// ```text
/// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓  ▓▓▓▓▓▓▓▓▓▓▓▓▓▓░░░░░░░░░░░░░░░░░░░
///       ⠋ 847 found                       ⠋ 423/847 sized
/// ```
pub struct DualProgressBar {
    scan_complete: bool,
    scan_count: usize,
    size_done: usize,
    size_total: usize,
    spinner_frame: usize,
}

impl DualProgressBar {
    pub const fn new() -> Self {
        Self {
            scan_complete: false,
            scan_count: 0,
            size_done: 0,
            size_total: 0,
            spinner_frame: 0,
        }
    }

    pub const fn scan_complete(mut self, complete: bool) -> Self {
        self.scan_complete = complete;
        self
    }

    pub const fn scan_count(mut self, count: usize) -> Self {
        self.scan_count = count;
        self
    }

    pub const fn size_progress(mut self, done: usize, total: usize) -> Self {
        self.size_done = done;
        self.size_total = total;
        self
    }

    pub const fn spinner_frame(mut self, frame: usize) -> Self {
        self.spinner_frame = frame;
        self
    }

    fn spinner_char(&self) -> char {
        const SPINNER: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
        SPINNER[self.spinner_frame % SPINNER.len()]
    }

    /// Render scan bar (full cyan while scanning, full green when complete)
    fn render_scan_bar(&self, width: usize) -> Vec<(char, Color)> {
        let color = if self.scan_complete {
            Color::Green
        } else {
            Color::Cyan
        };
        vec![('▓', color); width]
    }

    /// Render size bar (actual progress)
    fn render_size_bar(&self, width: usize) -> Vec<(char, Color)> {
        if self.size_total == 0 {
            vec![('░', Color::DarkGray); width]
        } else {
            #[allow(clippy::cast_precision_loss)]
            let ratio = self.size_done as f64 / self.size_total as f64;
            #[allow(
                clippy::cast_precision_loss,
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss
            )]
            let filled = (ratio * width as f64).round() as usize;
            let empty = width.saturating_sub(filled);

            let fill_color = if self.size_done >= self.size_total {
                Color::Green
            } else {
                Color::Cyan
            };

            let mut result = Vec::with_capacity(width);
            result.extend(vec![('▓', fill_color); filled]);
            result.extend(vec![('░', Color::DarkGray); empty]);
            result
        }
    }

    /// Render scan label: `⠋ X found` or `✓ X found`
    fn scan_label(&self) -> (String, Color) {
        if self.scan_complete {
            (format!("✓ {} found", self.scan_count), Color::Green)
        } else {
            (
                format!("{} {} found", self.spinner_char(), self.scan_count),
                Color::Cyan,
            )
        }
    }

    /// Render size label: `⠋ X/Y sized` or `✓ Y sized`
    fn size_label(&self) -> (String, Color) {
        if self.size_total == 0 {
            (String::new(), Color::DarkGray)
        } else if self.size_done >= self.size_total {
            (format!("✓ {} sized", self.size_total), Color::Green)
        } else {
            (
                format!(
                    "{} {}/{} sized",
                    self.spinner_char(),
                    self.size_done,
                    self.size_total
                ),
                Color::Yellow,
            )
        }
    }
}

impl Default for DualProgressBar {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for DualProgressBar {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height < 2 || area.width < 10 {
            return;
        }

        let width = area.width as usize;
        let half = width / 2;
        let gap = 2; // Space between the two bars
        let left_width = half.saturating_sub(gap / 2);
        let right_width = width.saturating_sub(half).saturating_sub(gap / 2);

        // Line 1: Two bars side by side
        // Left bar (scan)
        let scan_bar = self.render_scan_bar(left_width);
        for (i, (ch, color)) in scan_bar.into_iter().enumerate() {
            #[allow(clippy::cast_possible_truncation)]
            buf.set_string(
                area.x + i as u16,
                area.y,
                ch.to_string(),
                Style::default().fg(color),
            );
        }

        // Right bar (size)
        let size_bar = self.render_size_bar(right_width);
        let right_start = half + gap / 2;
        for (i, (ch, color)) in size_bar.into_iter().enumerate() {
            #[allow(clippy::cast_possible_truncation)]
            buf.set_string(
                area.x + (right_start + i) as u16,
                area.y,
                ch.to_string(),
                Style::default().fg(color),
            );
        }

        // Line 2: Labels centered under each bar
        let (scan_text, scan_color) = self.scan_label();
        let (size_text, size_color) = self.size_label();

        // Center scan label under left bar
        let scan_label = format!("{scan_text:^left_width$}");
        let scan_span = Span::styled(scan_label, Style::default().fg(scan_color));

        // Center size label under right bar
        let size_label = format!("{size_text:^right_width$}");
        let size_span = Span::styled(size_label, Style::default().fg(size_color));

        // Gap between labels
        let gap_span = Span::raw(" ".repeat(gap));

        let label_line = Line::from(vec![scan_span, gap_span, size_span]);
        #[allow(clippy::cast_possible_truncation)]
        buf.set_line(area.x, area.y + 1, &label_line, width as u16);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_bar_in_progress() {
        let bar = DualProgressBar::new().scan_complete(false).scan_count(100);

        let scan_bar = bar.render_scan_bar(40);
        assert_eq!(scan_bar.len(), 40);

        // All cyan while scanning
        assert!(scan_bar
            .iter()
            .all(|(c, color)| *c == '▓' && *color == Color::Cyan));
    }

    #[test]
    fn test_scan_bar_complete() {
        let bar = DualProgressBar::new().scan_complete(true).scan_count(100);

        let scan_bar = bar.render_scan_bar(40);

        // All green when complete
        assert!(scan_bar
            .iter()
            .all(|(c, color)| *c == '▓' && *color == Color::Green));
    }

    #[test]
    fn test_size_bar_empty() {
        let bar = DualProgressBar::new().size_progress(0, 0);

        let size_bar = bar.render_size_bar(40);
        assert_eq!(size_bar.len(), 40);

        // All grey when no sizing
        assert!(size_bar.iter().all(|(c, _)| *c == '░'));
    }

    #[test]
    fn test_size_bar_half() {
        let bar = DualProgressBar::new().size_progress(50, 100);

        let size_bar = bar.render_size_bar(40);
        assert_eq!(size_bar.len(), 40);

        // Should have mix of filled and empty
        let filled: Vec<_> = size_bar.iter().filter(|(c, _)| *c == '▓').collect();
        let empty: Vec<_> = size_bar.iter().filter(|(c, _)| *c == '░').collect();
        assert_eq!(filled.len(), 20);
        assert_eq!(empty.len(), 20);
    }

    #[test]
    fn test_size_bar_complete() {
        let bar = DualProgressBar::new().size_progress(100, 100);

        let size_bar = bar.render_size_bar(40);

        // All filled when complete
        assert!(size_bar.iter().all(|(c, _)| *c == '▓'));
    }

    #[test]
    fn test_scan_label_in_progress() {
        let bar = DualProgressBar::new()
            .scan_complete(false)
            .scan_count(42)
            .spinner_frame(0);

        let (label, color) = bar.scan_label();
        assert!(label.contains("42"));
        assert!(label.contains("found"));
        assert_eq!(color, Color::Cyan);
    }

    #[test]
    fn test_scan_label_complete() {
        let bar = DualProgressBar::new().scan_complete(true).scan_count(100);

        let (label, color) = bar.scan_label();
        assert!(label.contains("✓"));
        assert!(label.contains("100"));
        assert_eq!(color, Color::Green);
    }

    #[test]
    fn test_size_label_in_progress() {
        let bar = DualProgressBar::new()
            .size_progress(50, 100)
            .spinner_frame(0);

        let (label, color) = bar.size_label();
        assert!(label.contains("50/100"));
        assert_eq!(color, Color::Yellow);
    }

    #[test]
    fn test_size_label_complete() {
        let bar = DualProgressBar::new().size_progress(100, 100);

        let (label, color) = bar.size_label();
        assert!(label.contains("✓"));
        assert!(label.contains("100"));
        assert_eq!(color, Color::Green);
    }
}
