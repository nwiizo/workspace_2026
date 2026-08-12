use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use lazyssh::app::App;
use lazyssh::config::Host;
use lazyssh::input::{Action, handle_key};

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn app() -> App {
    App::new(vec![Host {
        alias: "prod".into(),
        host_name: Some("prod.example.com".into()),
        user: None,
        port: None,
        identity_files: Vec::new(),
        proxy_jump: None,
        source: PathBuf::from("config"),
        line: 1,
    }])
}

#[test]
fn search_mode_accepts_q_as_text_and_escape_clears_the_filter() {
    let mut app = app();

    assert_eq!(handle_key(&mut app, key(KeyCode::Char('/'))), Action::None);
    assert!(app.is_searching());
    assert_eq!(handle_key(&mut app, key(KeyCode::Char('q'))), Action::None);
    assert_eq!(app.query(), "q");

    assert_eq!(handle_key(&mut app, key(KeyCode::Esc)), Action::None);
    assert!(!app.is_searching());
    assert_eq!(app.query(), "");
}

#[test]
fn enter_connects_to_the_selected_alias_without_expanding_it() {
    let mut app = app();

    assert_eq!(
        handle_key(&mut app, key(KeyCode::Enter)),
        Action::Connect("prod".into())
    );
}
