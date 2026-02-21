use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixListener;
use std::sync::mpsc;
use std::thread;

use crate::history::CompletedTask;
use crate::i18n::Lang;
use crate::timer::{TimerConfig, TimerPhase, TimerStatus};

pub const SOCKET_PATH: &str = "/tmp/yasume.sock";

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "command")]
pub enum Command {
    #[serde(rename = "start")]
    Start,
    #[serde(rename = "pause")]
    Pause,
    #[serde(rename = "reset")]
    Reset,
    #[serde(rename = "skip")]
    Skip,
    #[serde(rename = "status")]
    Status,
    #[serde(rename = "set_task")]
    SetTask { name: String },
    #[serde(rename = "clear_task")]
    ClearTask,
    #[serde(rename = "set_times")]
    SetTimes {
        work: Option<u64>,
        short_break: Option<u64>,
        long_break: Option<u64>,
        auto_start_work: Option<bool>,
        auto_start_break: Option<bool>,
    },
    #[serde(rename = "history")]
    History { today_only: Option<bool> },
    #[serde(rename = "report")]
    Report { week: Option<bool> },
    #[serde(rename = "done")]
    Done,
    #[serde(rename = "log")]
    Log {
        name: String,
        duration: u64,
        at: Option<String>,
    },
    #[serde(rename = "quit")]
    Quit,
    #[serde(rename = "list")]
    List {
        from: Option<String>,
        to: Option<String>,
    },
    #[serde(rename = "set_lang")]
    SetLang { lang: Lang },
    #[serde(rename = "recent_tasks")]
    RecentTasks,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Response {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<StatusInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub history: Option<Vec<CompletedTask>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub report: Option<ReportInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recent_tasks: Option<Vec<String>>,
}

impl Response {
    pub fn ok(message: impl Into<String>) -> Self {
        Self {
            ok: true,
            message: Some(message.into()),
            status: None,
            history: None,
            report: None,
            recent_tasks: None,
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            ok: false,
            message: Some(message.into()),
            status: None,
            history: None,
            report: None,
            recent_tasks: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StatusInfo {
    pub phase: TimerPhase,
    pub status: TimerStatus,
    pub remaining: String,
    pub task: Option<String>,
    pub completed_sessions: u32,
    pub config: TimerConfig,
    pub today_pomodoros: usize,
    pub today_focus_minutes: u64,
    pub lang: Lang,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReportInfo {
    pub period: String,
    pub total_pomodoros: usize,
    pub total_focus_minutes: u64,
    pub unique_tasks: Vec<String>,
    pub daily_breakdown: Vec<DailyBreakdown>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DailyBreakdown {
    pub date: String,
    pub pomodoros: usize,
    pub focus_minutes: u64,
}

pub type CommandSender = mpsc::Sender<(Command, mpsc::Sender<Response>)>;
pub type CommandReceiver = mpsc::Receiver<(Command, mpsc::Sender<Response>)>;

pub fn create_channel() -> (CommandSender, CommandReceiver) {
    mpsc::channel()
}

pub fn start_listener(cmd_tx: CommandSender) {
    // Clean up stale socket
    if std::path::Path::new(SOCKET_PATH).exists() {
        // Try connecting to check if another instance is running
        if std::os::unix::net::UnixStream::connect(SOCKET_PATH).is_ok() {
            eprintln!("Another instance is already running on {SOCKET_PATH}");
            std::process::exit(1);
        }
        // Stale socket, remove it
        let _ = std::fs::remove_file(SOCKET_PATH);
    }

    let listener = match UnixListener::bind(SOCKET_PATH) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Failed to bind IPC socket: {e}");
            return;
        }
    };

    thread::spawn(move || {
        for stream in listener.incoming() {
            let mut stream = match stream {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("IPC accept error: {e}");
                    continue;
                }
            };

            let reader = BufReader::new(match stream.try_clone() {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("IPC clone error: {e}");
                    continue;
                }
            });

            let mut lines = reader.lines();
            let line = match lines.next() {
                Some(Ok(l)) => l,
                _ => continue,
            };

            let cmd: Command = match serde_json::from_str(&line) {
                Ok(c) => c,
                Err(e) => {
                    let resp = Response::error(format!("Invalid command: {e}"));
                    let _ = writeln!(
                        stream,
                        "{}",
                        serde_json::to_string(&resp).unwrap_or_default()
                    );
                    continue;
                }
            };

            let (resp_tx, resp_rx) = mpsc::channel();
            if cmd_tx.send((cmd, resp_tx)).is_err() {
                break; // Main thread dropped, exit
            }

            match resp_rx.recv() {
                Ok(resp) => {
                    let _ = writeln!(
                        stream,
                        "{}",
                        serde_json::to_string(&resp).unwrap_or_default()
                    );
                }
                Err(_) => {
                    let resp = Response::error("No response from timer");
                    let _ = writeln!(
                        stream,
                        "{}",
                        serde_json::to_string(&resp).unwrap_or_default()
                    );
                }
            }
        }
    });
}

pub fn cleanup_socket() {
    let _ = std::fs::remove_file(SOCKET_PATH);
}
