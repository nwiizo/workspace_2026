use chrono::{Datelike, Local};
use clap::{Parser, Subcommand};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::process::Command as ProcessCommand;
use std::thread;
use std::time::Duration;

use yasume::i18n::{Lang, strings};
use yasume::ipc::{Command, Response, SOCKET_PATH};
use yasume::timer::TimerPhase;

#[derive(Parser)]
#[command(
    name = "yasume-ctl",
    about = "yasume — Pomodoro timer control CLI\n\nControls the yasume timer GUI. Use 'status' to see current phase/task/time.\nPhases cycle: Work(25m) → ShortBreak(5m) → ... → LongBreak(15m) after 4 sessions."
)]
struct Cli {
    #[command(subcommand)]
    command: CtlCommand,
}

#[derive(Subcommand)]
enum CtlCommand {
    /// Start the timer. Auto-launches yasume GUI if not running.
    /// Example: yasume-ctl start
    Start,
    /// Pause a running timer. Resume with 'start'.
    Pause,
    /// Reset the current phase timer to its full duration (e.g. 25:00).
    Reset,
    /// Skip to the next phase (Work→Break or Break→Work). Counts as completed.
    Skip,
    /// Show current timer state: phase, remaining time, task, today's stats.
    /// Useful for scripts and AI agents to understand what you're working on.
    Status,
    /// Set or clear the current task name displayed on the timer.
    /// Example: yasume-ctl task "Write report" / yasume-ctl task --clear
    Task {
        /// Task name to set (omit to clear)
        name: Option<String>,
        /// Clear current task name
        #[arg(long)]
        clear: bool,
    },
    /// Configure timer durations and auto-start behavior.
    /// Example: yasume-ctl times --work 50 --short-break 10
    Times {
        /// Work duration in minutes (default: 25)
        #[arg(long)]
        work: Option<u64>,
        /// Short break duration in minutes (default: 5)
        #[arg(long)]
        short_break: Option<u64>,
        /// Long break duration in minutes (default: 15)
        #[arg(long)]
        long_break: Option<u64>,
        /// Auto-start work phase after break ends
        #[arg(long)]
        auto_start_work: Option<bool>,
        /// Auto-start break phase after work ends
        #[arg(long)]
        auto_start_break: Option<bool>,
    },
    /// Show raw completed task history (JSON-serializable).
    History {
        /// Show only today's tasks (default: all)
        #[arg(long)]
        today: bool,
    },
    /// Show productivity summary: total pomodoros, focus minutes, unique tasks.
    /// Example: yasume-ctl report / yasume-ctl report --week
    Report {
        /// Show weekly report (default: today only)
        #[arg(long)]
        week: bool,
    },
    /// Finish the current session early and record actual elapsed time.
    /// Useful when you complete a task before the timer runs out.
    Done,
    /// Log a completed task retroactively (not via timer).
    /// Example: yasume-ctl log "Meeting" -d 60 --at 14:00
    Log {
        /// Task name
        name: String,
        /// Duration in minutes
        #[arg(long, short)]
        duration: u64,
        /// Start time (HH:MM format, defaults to now minus duration)
        #[arg(long)]
        at: Option<String>,
    },
    /// Quit the yasume GUI application.
    Quit,
    /// List history entries grouped by date with summary.
    /// Example: yasume-ctl list / yasume-ctl list --week / yasume-ctl list --from 2026-02-10 --to 2026-02-14
    List {
        /// Show this week's entries
        #[arg(long)]
        week: bool,
        /// Start date (YYYY-MM-DD)
        #[arg(long)]
        from: Option<String>,
        /// End date (YYYY-MM-DD)
        #[arg(long)]
        to: Option<String>,
    },
    /// Switch display language.
    /// Example: yasume-ctl lang ja / yasume-ctl lang en
    Lang {
        /// Language code: "ja" (Japanese) or "en" (English)
        lang: String,
    },
    /// Show recent task names from the past 7 days.
    /// Useful for quickly picking a task to continue working on.
    Tasks,
}

fn send_command(cmd: &Command) -> Result<Response, Box<dyn std::error::Error>> {
    let mut stream = UnixStream::connect(SOCKET_PATH)?;
    let json = serde_json::to_string(cmd)?;
    writeln!(stream, "{json}")?;
    stream.flush()?;

    let reader = BufReader::new(&stream);
    let mut lines = reader.lines();
    let line = lines.next().ok_or("No response")??;
    let resp: Response = serde_json::from_str(&line)?;
    Ok(resp)
}

fn try_auto_start() -> bool {
    let self_path = std::env::current_exe().ok();
    let candidates: Vec<std::path::PathBuf> = if let Some(ref self_path) = self_path {
        if let Some(dir) = self_path.parent() {
            vec![dir.join("yasume")]
        } else {
            vec![]
        }
    } else {
        vec![]
    };

    for candidate in &candidates {
        if candidate.exists()
            && ProcessCommand::new(candidate)
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .is_ok()
        {
            return true;
        }
    }

    ProcessCommand::new("yasume")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .is_ok()
}

fn send_with_auto_start(cmd: &Command) -> Result<Response, Box<dyn std::error::Error>> {
    match send_command(cmd) {
        Ok(resp) => Ok(resp),
        Err(_) => {
            if !try_auto_start() {
                return Err("Failed to start yasume".into());
            }

            for _ in 0..30 {
                thread::sleep(Duration::from_millis(100));
                if let Ok(resp) = send_command(cmd) {
                    return Ok(resp);
                }
            }
            Err("Failed to connect to yasume".into())
        }
    }
}

fn weekday_ja(date: chrono::NaiveDate) -> &'static str {
    match date.weekday() {
        chrono::Weekday::Mon => "月",
        chrono::Weekday::Tue => "火",
        chrono::Weekday::Wed => "水",
        chrono::Weekday::Thu => "木",
        chrono::Weekday::Fri => "金",
        chrono::Weekday::Sat => "土",
        chrono::Weekday::Sun => "日",
    }
}

fn weekday_en(date: chrono::NaiveDate) -> &'static str {
    match date.weekday() {
        chrono::Weekday::Mon => "Mon",
        chrono::Weekday::Tue => "Tue",
        chrono::Weekday::Wed => "Wed",
        chrono::Weekday::Thu => "Thu",
        chrono::Weekday::Fri => "Fri",
        chrono::Weekday::Sat => "Sat",
        chrono::Weekday::Sun => "Sun",
    }
}

fn main() {
    let cli = Cli::parse();

    let (cmd, use_auto_start) = match cli.command {
        CtlCommand::Start => (Command::Start, true),
        CtlCommand::Pause => (Command::Pause, false),
        CtlCommand::Reset => (Command::Reset, false),
        CtlCommand::Skip => (Command::Skip, false),
        CtlCommand::Status => (Command::Status, false),
        CtlCommand::Task { name, clear } => {
            let cmd = match name {
                Some(name) if !clear => Command::SetTask { name },
                _ => Command::ClearTask,
            };
            (cmd, false)
        }
        CtlCommand::Times {
            work,
            short_break,
            long_break,
            auto_start_work,
            auto_start_break,
        } => (
            Command::SetTimes {
                work,
                short_break,
                long_break,
                auto_start_work,
                auto_start_break,
            },
            false,
        ),
        CtlCommand::History { today } => (
            Command::History {
                today_only: Some(today),
            },
            false,
        ),
        CtlCommand::Report { week } => (Command::Report { week: Some(week) }, false),
        CtlCommand::Done => (Command::Done, false),
        CtlCommand::Log { name, duration, at } => (Command::Log { name, duration, at }, false),
        CtlCommand::Quit => (Command::Quit, false),
        CtlCommand::List { week, from, to } => {
            let (from, to) = if week {
                let today = Local::now().date_naive();
                let days_from_monday = today.weekday().num_days_from_monday();
                let week_start = today - chrono::Duration::days(i64::from(days_from_monday));
                (
                    Some(week_start.format("%Y-%m-%d").to_string()),
                    Some(today.format("%Y-%m-%d").to_string()),
                )
            } else {
                (from, to)
            };
            (Command::List { from, to }, false)
        }
        CtlCommand::Lang { lang: lang_str } => {
            let lang = match lang_str.as_str() {
                "ja" => Lang::Ja,
                "en" => Lang::En,
                _ => {
                    eprintln!("Unknown language: {lang_str} (use 'ja' or 'en')");
                    std::process::exit(1);
                }
            };
            (Command::SetLang { lang }, false)
        }
        CtlCommand::Tasks => (Command::RecentTasks, false),
    };

    let result = if use_auto_start {
        send_with_auto_start(&cmd)
    } else {
        send_command(&cmd)
    };

    match result {
        Ok(resp) => {
            if let Some(status) = &resp.status {
                let lang = status.lang;
                let s = strings(lang);
                let status_label = match status.status {
                    yasume::timer::TimerStatus::Idle => s.status_idle,
                    yasume::timer::TimerStatus::Running => s.status_running,
                    yasume::timer::TimerStatus::Paused => s.status_paused,
                    yasume::timer::TimerStatus::Finished => s.status_finished,
                };
                let phase_label = status.phase.label_with(lang);
                println!(
                    "Phase: {phase_label} | Status: {status_label} | Remaining: {}",
                    status.remaining
                );
                if let Some(task) = &status.task {
                    println!("Task: {task}");
                }
                println!(
                    "Session: {}/4 | {}",
                    status.completed_sessions,
                    s.format_stats(status.today_pomodoros, status.today_focus_minutes),
                );
                println!(
                    "Config: {}={}m {}={}m {}={}m auto_work={} auto_break={}",
                    s.work,
                    status.config.work_minutes,
                    s.short_break,
                    status.config.short_break_minutes,
                    s.long_break,
                    status.config.long_break_minutes,
                    status.config.auto_start_work,
                    status.config.auto_start_break,
                );
            } else if let Some(report) = &resp.report {
                println!("=== {} ===", report.period);
                println!("{}", report.total_pomodoros);
                println!("{}min", report.total_focus_minutes);
                if !report.unique_tasks.is_empty() {
                    println!("{}", report.unique_tasks.join(", "));
                }
                if report.daily_breakdown.len() > 1 {
                    println!();
                    for day in &report.daily_breakdown {
                        println!(
                            "  {} : {} / {}min",
                            day.date, day.pomodoros, day.focus_minutes
                        );
                    }
                }
            } else if let Some(history) = &resp.history {
                if history.is_empty() {
                    println!("{}", strings(Lang::default()).cli_no_history);
                } else {
                    print_history_grouped(history, Lang::default());
                }
            } else if let Some(tasks) = &resp.recent_tasks {
                if tasks.is_empty() {
                    println!("{}", strings(Lang::default()).cli_no_history);
                } else {
                    println!("Recent tasks (7 days):");
                    for (i, name) in tasks.iter().enumerate() {
                        println!("  {}. {name}", i + 1);
                    }
                }
            } else if let Some(msg) = &resp.message {
                println!("{msg}");
            }

            if !resp.ok {
                std::process::exit(1);
            }
        }
        Err(e) => {
            eprintln!("Error: {e}");
            eprintln!("{}", strings(Lang::default()).cli_error_not_running);
            std::process::exit(1);
        }
    }
}

fn print_history_grouped(history: &[yasume::history::CompletedTask], lang: Lang) {
    use std::collections::BTreeMap;

    let mut by_date: BTreeMap<chrono::NaiveDate, Vec<&yasume::history::CompletedTask>> =
        BTreeMap::new();
    for task in history {
        let date = task.completed_at.date_naive();
        by_date.entry(date).or_default().push(task);
    }

    for (date, tasks) in by_date.iter().rev() {
        let wd = match lang {
            Lang::Ja => weekday_ja(*date),
            Lang::En => weekday_en(*date),
        };

        // Day summary
        let day_pomodoros = tasks.iter().filter(|t| t.phase == TimerPhase::Work).count();
        let day_minutes: u64 = tasks
            .iter()
            .filter(|t| t.phase == TimerPhase::Work)
            .map(|t| t.duration_minutes)
            .sum();
        let summary = match lang {
            Lang::Ja => format!("{}回 / {}分", day_pomodoros, day_minutes),
            Lang::En => format!("{} pomodoros / {}min", day_pomodoros, day_minutes),
        };

        println!("{} ({})  [{}]", date.format("%Y-%m-%d"), wd, summary);
        for task in tasks {
            let time = task.started_at.format("%H:%M");
            let phase = task.phase.label_with(lang);
            let duration = match lang {
                Lang::Ja => format!("{}分", task.duration_minutes),
                Lang::En => format!("{}m", task.duration_minutes),
            };
            let name = task.task_name.as_deref().unwrap_or("");
            if task.phase == TimerPhase::Work {
                println!("  {time}  {phase}  {duration:>5}  {name}");
            } else {
                println!("  {time}  {phase}  {duration:>5}");
            }
        }
        println!();
    }

    // Overall summary
    let total_pomodoros = history
        .iter()
        .filter(|t| t.phase == TimerPhase::Work)
        .count();
    let total_minutes: u64 = history
        .iter()
        .filter(|t| t.phase == TimerPhase::Work)
        .map(|t| t.duration_minutes)
        .sum();
    match lang {
        Lang::Ja => println!("合計: {}回 / {}分", total_pomodoros, total_minutes),
        Lang::En => println!(
            "Total: {} pomodoros / {}min",
            total_pomodoros, total_minutes
        ),
    }
}
