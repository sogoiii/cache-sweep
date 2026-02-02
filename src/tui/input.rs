use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::app::{App, Mode, Panel, SortOrder};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Continue,
    Quit,
    Delete,
    DeleteSelected,
    OpenInExplorer,
}

pub fn handle_key(key: KeyEvent, app: &mut App) -> Action {
    match app.mode {
        Mode::Search => handle_search_key(key, app),
        Mode::MultiSelect => handle_multi_select_key(key, app),
        Mode::Normal => handle_normal_key(key, app),
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
            app.move_cursor(-(app.visible_height as isize));
            Action::Continue
        }
        KeyCode::PageDown | KeyCode::Char('d') => {
            app.move_cursor(app.visible_height as isize);
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
                if app.cursor >= app.visible_height {
                    app.scroll_offset = app.cursor - app.visible_height + 1;
                }
            }
            Action::Continue
        }

        // Panel navigation
        KeyCode::Left | KeyCode::Char('h') => {
            app.panel = match app.panel {
                Panel::Results => Panel::Options,
                Panel::Info => Panel::Results,
                Panel::Options => Panel::Help,
                Panel::Help => Panel::Options,
            };
            Action::Continue
        }
        KeyCode::Right | KeyCode::Char('l') => {
            app.panel = match app.panel {
                Panel::Results => Panel::Info,
                Panel::Info => Panel::Results,
                Panel::Options => Panel::Results,
                Panel::Help => Panel::Options,
            };
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
            app.move_cursor(-(app.visible_height as isize));
            Action::Continue
        }
        KeyCode::PageDown | KeyCode::Char('d') => {
            app.move_cursor(app.visible_height as isize);
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
            if !app.selected_indices.is_empty() {
                Action::DeleteSelected
            } else {
                Action::Continue
            }
        }

        _ => Action::Continue,
    }
}
