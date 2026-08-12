use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::app::App;

#[derive(Debug, Eq, PartialEq)]
pub enum Action {
    None,
    Quit,
    Connect(String),
    Reload,
}

pub fn handle_key(app: &mut App, key: KeyEvent) -> Action {
    if key.kind == KeyEventKind::Release {
        return Action::None;
    }

    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        return Action::Quit;
    }

    if app.is_showing_help() {
        if matches!(
            key.code,
            KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q')
        ) {
            app.toggle_help();
        }
        return Action::None;
    }

    if app.is_searching() {
        return handle_search_key(app, key);
    }

    match key.code {
        KeyCode::Char('q') => Action::Quit,
        KeyCode::Char('/') => {
            app.start_search();
            Action::None
        }
        KeyCode::Char('j') | KeyCode::Down => {
            app.select_next();
            Action::None
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.select_previous();
            Action::None
        }
        KeyCode::Char('g') | KeyCode::Home => {
            app.select_first();
            Action::None
        }
        KeyCode::Char('G') | KeyCode::End => {
            app.select_last();
            Action::None
        }
        KeyCode::Char('r') => Action::Reload,
        KeyCode::Char('?') => {
            app.toggle_help();
            Action::None
        }
        KeyCode::Enter => app
            .selected_host()
            .map_or(Action::None, |host| Action::Connect(host.alias.clone())),
        _ => Action::None,
    }
}

fn handle_search_key(app: &mut App, key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Esc => app.cancel_search(),
        KeyCode::Enter => app.finish_search(),
        KeyCode::Backspace => app.pop_query(),
        KeyCode::Char(character)
            if !key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
        {
            app.push_query(character);
        }
        _ => {}
    }
    Action::None
}
