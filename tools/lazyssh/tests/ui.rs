use std::path::{Path, PathBuf};

use lazyssh::app::App;
use lazyssh::config::Host;
use ratatui::Terminal;
use ratatui::backend::TestBackend;

fn app() -> App {
    App::new(vec![Host {
        alias: "prod-web".into(),
        host_name: Some("10.0.0.8".into()),
        user: Some("deploy".into()),
        port: Some(2222),
        identity_files: vec!["~/.ssh/prod".into()],
        proxy_jump: Some("bastion".into()),
        source: PathBuf::from("fixtures/config"),
        line: 3,
    }])
}

#[test]
fn dashboard_renders_the_selected_host_and_connection_details() {
    let mut terminal = Terminal::new(TestBackend::new(100, 24)).expect("test terminal");

    terminal
        .draw(|frame| lazyssh::ui::render(frame, &app(), Path::new("fixtures/config")))
        .expect("render dashboard");

    let content = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(content.contains("lazyssh"));
    assert!(content.contains("prod-web"));
    assert!(content.contains("deploy@10.0.0.8:2222"));
    assert!(content.contains("ProxyJump"));
}

#[test]
fn help_overlay_renders_on_a_small_terminal_without_panicking() {
    let mut app = app();
    app.toggle_help();
    let mut terminal = Terminal::new(TestBackend::new(24, 8)).expect("test terminal");

    terminal
        .draw(|frame| lazyssh::ui::render(frame, &app, Path::new("config")))
        .expect("render help");
}
