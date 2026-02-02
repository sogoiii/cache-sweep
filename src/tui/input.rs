use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::app::{App, Mode, Panel, SortOrder};
use super::panels;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Continue,
    Quit,
    Delete,
    DeleteSelected,
    OpenInExplorer,
}

pub fn handle_key(key: KeyEvent, app: &mut App) -> Action {
    // Analytics panel has its own handling regardless of mode
    if app.panel == Panel::Analytics {
        return handle_analytics_key(key, app);
    }

    match app.mode {
        Mode::Search => handle_search_key(key, app),
        Mode::MultiSelect => handle_multi_select_key(key, app),
        Mode::Normal => handle_normal_key(key, app),
    }
}

fn handle_analytics_key(key: KeyEvent, app: &mut App) -> Action {
    match key.code {
        KeyCode::Char('a') | KeyCode::Esc => {
            app.panel = Panel::Results;
            Action::Continue
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app.analytics_scroll = app.analytics_scroll.saturating_sub(1);
            Action::Continue
        }
        KeyCode::Down | KeyCode::Char('j') => {
            let max_scroll = panels::content_line_count(app).saturating_sub(10);
            app.analytics_scroll = (app.analytics_scroll + 1).min(max_scroll);
            Action::Continue
        }
        KeyCode::PageUp | KeyCode::Char('u') => {
            app.analytics_scroll = app.analytics_scroll.saturating_sub(10);
            Action::Continue
        }
        KeyCode::PageDown | KeyCode::Char('d') => {
            let max_scroll = panels::content_line_count(app).saturating_sub(10);
            app.analytics_scroll = (app.analytics_scroll + 10).min(max_scroll);
            Action::Continue
        }
        KeyCode::Home => {
            app.analytics_scroll = 0;
            Action::Continue
        }
        KeyCode::End => {
            let max_scroll = panels::content_line_count(app).saturating_sub(10);
            app.analytics_scroll = max_scroll;
            Action::Continue
        }
        KeyCode::Char('q') => Action::Quit,
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => Action::Quit,
        _ => Action::Continue,
    }
}

fn handle_normal_key(key: KeyEvent, app: &mut App) -> Action {
    match key.code {
        // Quit
        KeyCode::Char('q') | KeyCode::Esc => Action::Quit,
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => Action::Quit,

        // Navigation
        KeyCode::Up | KeyCode::Char('k') => {
            app.move_cursor(-1);
            Action::Continue
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.move_cursor(1);
            Action::Continue
        }
        KeyCode::PageUp | KeyCode::Char('u') => {
            app.move_cursor_by_page(false);
            Action::Continue
        }
        KeyCode::PageDown | KeyCode::Char('d') => {
            app.move_cursor_by_page(true);
            Action::Continue
        }
        KeyCode::Home => {
            app.cursor = 0;
            app.scroll_offset = 0;
            Action::Continue
        }
        KeyCode::End => {
            if !app.filtered_indices.is_empty() {
                app.cursor = app.filtered_indices.len() - 1;
                if app.cursor >= app.visible_height - 2 {
                    // Keep cursor 2 rows above the bottom (accounts for UI chrome)
                    app.scroll_offset = app.cursor - app.visible_height + 3;
                }
            }
            Action::Continue
        }

        // Panel navigation
        KeyCode::Left | KeyCode::Char('h') => {
            app.panel = match app.panel {
                Panel::Info => Panel::Results,
                Panel::Options => Panel::Help,
                Panel::Results | Panel::Help | Panel::Analytics => Panel::Options,
            };
            Action::Continue
        }
        KeyCode::Right | KeyCode::Char('l') => {
            app.panel = match app.panel {
                Panel::Results => Panel::Info,
                Panel::Help => Panel::Options,
                Panel::Info | Panel::Options | Panel::Analytics => Panel::Results,
            };
            Action::Continue
        }

        // Analytics panel toggle
        KeyCode::Char('a') if app.panel != Panel::Info => {
            app.panel = Panel::Analytics;
            app.analytics_scroll = 0; // Reset scroll on open
            Action::Continue
        }

        // Actions (disabled on Info panel)
        KeyCode::Char(' ') | KeyCode::Delete if app.panel != Panel::Info => Action::Delete,
        KeyCode::Char('/') if app.panel != Panel::Info => {
            app.mode = Mode::Search;
            app.search_query.clear();
            Action::Continue
        }
        KeyCode::Char('t') if app.panel != Panel::Info => {
            app.mode = Mode::MultiSelect;
            Action::Continue
        }
        KeyCode::Char('s') if app.panel != Panel::Info => {
            app.sort_order = match app.sort_order {
                SortOrder::Size => SortOrder::Path,
                SortOrder::Path => SortOrder::Age,
                SortOrder::Age => SortOrder::Size,
            };
            app.apply_sort_and_filter();
            Action::Continue
        }

        // Open in file explorer (Info panel only)
        KeyCode::Char('o') if app.panel == Panel::Info => Action::OpenInExplorer,

        KeyCode::Char('e') => {
            // Toggle error display (handled in UI)
            Action::Continue
        }

        _ => Action::Continue,
    }
}

fn handle_search_key(key: KeyEvent, app: &mut App) -> Action {
    match key.code {
        KeyCode::Esc => {
            app.mode = Mode::Normal;
            app.search_query.clear();
            app.needs_filter = true;
            Action::Continue
        }
        KeyCode::Enter => {
            app.mode = Mode::Normal;
            app.needs_filter = true;
            Action::Continue
        }
        KeyCode::Backspace => {
            app.search_query.pop();
            app.needs_filter = true;
            Action::Continue
        }
        KeyCode::Char(c) => {
            app.search_query.push(c);
            app.needs_filter = true;
            Action::Continue
        }
        _ => Action::Continue,
    }
}

fn handle_multi_select_key(key: KeyEvent, app: &mut App) -> Action {
    match key.code {
        KeyCode::Esc | KeyCode::Char('t') => {
            app.mode = Mode::Normal;
            app.deselect_all();
            Action::Continue
        }
        KeyCode::Char('q') => Action::Quit,

        // Navigation
        KeyCode::Up | KeyCode::Char('k') => {
            app.move_cursor(-1);
            Action::Continue
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.move_cursor(1);
            Action::Continue
        }
        KeyCode::PageUp | KeyCode::Char('u') => {
            app.move_cursor_by_page(false);
            Action::Continue
        }
        KeyCode::PageDown | KeyCode::Char('d') => {
            app.move_cursor_by_page(true);
            Action::Continue
        }

        // Selection
        KeyCode::Char(' ') => {
            app.toggle_selection();
            Action::Continue
        }
        KeyCode::Char('a') => {
            if app.selected_indices.is_empty() {
                app.select_all();
            } else {
                app.deselect_all();
            }
            Action::Continue
        }
        KeyCode::Enter => {
            if app.selected_indices.is_empty() {
                Action::Continue
            } else {
                Action::DeleteSelected
            }
        }

        _ => Action::Continue,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn key_ctrl(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::CONTROL)
    }

    fn app_in_analytics() -> App {
        let mut app = App::new(false, SortOrder::Size);
        app.panel = Panel::Analytics;
        app.analytics_scroll = 5;
        app
    }

    fn app_in_results() -> App {
        App::new(false, SortOrder::Size)
    }

    // === Analytics panel key handling ===

    #[test]
    fn test_analytics_a_returns_to_results() {
        let mut app = app_in_analytics();
        let action = handle_key(key(KeyCode::Char('a')), &mut app);

        assert_eq!(action, Action::Continue);
        assert_eq!(app.panel, Panel::Results);
    }

    #[test]
    fn test_analytics_esc_returns_to_results() {
        let mut app = app_in_analytics();
        let action = handle_key(key(KeyCode::Esc), &mut app);

        assert_eq!(action, Action::Continue);
        assert_eq!(app.panel, Panel::Results);
    }

    #[test]
    fn test_analytics_q_quits() {
        let mut app = app_in_analytics();
        let action = handle_key(key(KeyCode::Char('q')), &mut app);

        assert_eq!(action, Action::Quit);
    }

    #[test]
    fn test_analytics_ctrl_c_quits() {
        let mut app = app_in_analytics();
        let action = handle_key(key_ctrl(KeyCode::Char('c')), &mut app);

        assert_eq!(action, Action::Quit);
    }

    #[test]
    fn test_analytics_up_scrolls_up() {
        let mut app = app_in_analytics();
        app.analytics_scroll = 5;

        handle_key(key(KeyCode::Up), &mut app);
        assert_eq!(app.analytics_scroll, 4);
    }

    #[test]
    fn test_analytics_k_scrolls_up() {
        let mut app = app_in_analytics();
        app.analytics_scroll = 5;

        handle_key(key(KeyCode::Char('k')), &mut app);
        assert_eq!(app.analytics_scroll, 4);
    }

    #[test]
    fn test_analytics_scroll_up_stops_at_zero() {
        let mut app = app_in_analytics();
        app.analytics_scroll = 0;

        handle_key(key(KeyCode::Up), &mut app);
        assert_eq!(app.analytics_scroll, 0);
    }

    #[test]
    fn test_analytics_down_scrolls_down() {
        let mut app = app_in_analytics();
        app.analytics_scroll = 0;

        handle_key(key(KeyCode::Down), &mut app);
        assert_eq!(app.analytics_scroll, 1);
    }

    #[test]
    fn test_analytics_j_scrolls_down() {
        let mut app = app_in_analytics();
        app.analytics_scroll = 0;

        handle_key(key(KeyCode::Char('j')), &mut app);
        assert_eq!(app.analytics_scroll, 1);
    }

    #[test]
    fn test_analytics_page_up_scrolls_by_10() {
        let mut app = app_in_analytics();
        app.analytics_scroll = 15;

        handle_key(key(KeyCode::PageUp), &mut app);
        assert_eq!(app.analytics_scroll, 5);
    }

    #[test]
    fn test_analytics_u_scrolls_by_10() {
        let mut app = app_in_analytics();
        app.analytics_scroll = 15;

        handle_key(key(KeyCode::Char('u')), &mut app);
        assert_eq!(app.analytics_scroll, 5);
    }

    #[test]
    fn test_analytics_home_goes_to_top() {
        let mut app = app_in_analytics();
        app.analytics_scroll = 50;

        handle_key(key(KeyCode::Home), &mut app);
        assert_eq!(app.analytics_scroll, 0);
    }

    // === Opening analytics from Results ===

    #[test]
    fn test_results_a_opens_analytics() {
        let mut app = app_in_results();
        assert_eq!(app.panel, Panel::Results);

        handle_key(key(KeyCode::Char('a')), &mut app);

        assert_eq!(app.panel, Panel::Analytics);
        assert_eq!(app.analytics_scroll, 0);
    }

    #[test]
    fn test_results_a_resets_scroll() {
        let mut app = app_in_results();
        app.analytics_scroll = 50;

        handle_key(key(KeyCode::Char('a')), &mut app);

        assert_eq!(app.analytics_scroll, 0);
    }

    #[test]
    fn test_info_panel_a_does_not_open_analytics() {
        let mut app = app_in_results();
        app.panel = Panel::Info;

        handle_key(key(KeyCode::Char('a')), &mut app);

        assert_eq!(app.panel, Panel::Info);
    }
}
