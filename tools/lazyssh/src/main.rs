use std::error::Error;
use std::io::{self, IsTerminal, Stdout};
use std::path::{Path, PathBuf};

use crossterm::event::{self, Event};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use lazyssh::app::App;
use lazyssh::cli::{self, Outcome};
use lazyssh::config;
use lazyssh::connection::ssh_command;
use lazyssh::input::{Action, handle_key};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

const USAGE: &str = "lazyssh — keyboard-first OpenSSH host navigator

Usage: lazyssh [OPTIONS]

Options:
  -c, --config <PATH>  SSH config file [default: ~/.ssh/config]
  -h, --help           Print help
  -V, --version        Print version";

fn main() {
    if let Err(error) = run() {
        eprintln!("lazyssh: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let default_config = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("~"))
        .join(".ssh/config");
    let config_path = match cli::parse(std::env::args_os(), default_config) {
        Ok(Outcome::Help) => {
            println!("{USAGE}");
            return Ok(());
        }
        Ok(Outcome::Version) => {
            println!("lazyssh {}", env!("CARGO_PKG_VERSION"));
            return Ok(());
        }
        Ok(Outcome::Run(cli)) => cli.config,
        Err(error) => return Err(format!("{error}\n\n{USAGE}").into()),
    };

    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err("an interactive terminal is required".into());
    }

    let hosts = load_hosts(&config_path)?;
    let mut app = App::new(hosts);
    let mut terminal = TerminalSession::new()?;

    loop {
        terminal.draw(&app, &config_path)?;
        let Event::Key(key) = event::read()? else {
            continue;
        };

        match handle_key(&mut app, key) {
            Action::None => {}
            Action::Quit => break,
            Action::Reload => match load_hosts(&config_path) {
                Ok(hosts) => {
                    let count = hosts.len();
                    app.replace_hosts(hosts);
                    app.set_status(format!("Reloaded {count} hosts"));
                }
                Err(error) => app.set_status(format!("Reload failed: {error}")),
            },
            Action::Connect(alias) => {
                terminal.suspend()?;
                let result = ssh_command(&config_path, &alias).status();
                terminal.resume()?;
                match result {
                    Ok(status) if status.success() => {
                        app.set_status(format!("Connection to {alias} closed"));
                    }
                    Ok(status) => app.set_status(format!(
                        "ssh {alias} exited with {}",
                        status
                            .code()
                            .map_or_else(|| "a signal".into(), |code| format!("status {code}"))
                    )),
                    Err(error) => app.set_status(format!("Could not start ssh: {error}")),
                }
            }
        }
    }

    Ok(())
}

fn load_hosts(path: &Path) -> io::Result<Vec<config::Host>> {
    config::load(path).map_err(|error| {
        io::Error::new(
            error.kind(),
            format!("could not read {}: {error}", path.display()),
        )
    })
}

type Tui = Terminal<CrosstermBackend<Stdout>>;

struct TerminalSession {
    terminal: Tui,
    active: bool,
}

impl TerminalSession {
    fn new() -> io::Result<Self> {
        let backend = CrosstermBackend::new(io::stdout());
        let terminal = Terminal::new(backend)?;
        let mut session = Self {
            terminal,
            active: false,
        };
        session.resume()?;
        Ok(session)
    }

    fn draw(&mut self, app: &App, config_path: &Path) -> io::Result<()> {
        self.terminal
            .draw(|frame| lazyssh::ui::render(frame, app, config_path))?;
        Ok(())
    }

    fn suspend(&mut self) -> io::Result<()> {
        if !self.active {
            return Ok(());
        }
        disable_raw_mode()?;
        execute!(self.terminal.backend_mut(), LeaveAlternateScreen)?;
        self.terminal.show_cursor()?;
        self.active = false;
        Ok(())
    }

    fn resume(&mut self) -> io::Result<()> {
        if self.active {
            return Ok(());
        }
        enable_raw_mode()?;
        if let Err(error) = execute!(self.terminal.backend_mut(), EnterAlternateScreen) {
            let _ = disable_raw_mode();
            return Err(error);
        }
        self.active = true;
        let size = self.terminal.size()?;
        self.terminal.resize(size.into())?;
        Ok(())
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        if self.active {
            let _ = disable_raw_mode();
            let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
            let _ = self.terminal.show_cursor();
        }
    }
}
