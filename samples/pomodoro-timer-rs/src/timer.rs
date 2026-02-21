use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

const SESSIONS_BEFORE_LONG_BREAK: u32 = 4;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimerConfig {
    pub work_minutes: u64,
    pub short_break_minutes: u64,
    pub long_break_minutes: u64,
    pub auto_start_work: bool,
    pub auto_start_break: bool,
}

impl Default for TimerConfig {
    fn default() -> Self {
        Self {
            work_minutes: 25,
            short_break_minutes: 5,
            long_break_minutes: 15,
            auto_start_work: false,
            auto_start_break: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimerPhase {
    Work,
    ShortBreak,
    LongBreak,
}

impl TimerPhase {
    pub fn duration(self, config: &TimerConfig) -> Duration {
        match self {
            Self::Work => Duration::from_secs(config.work_minutes * 60),
            Self::ShortBreak => Duration::from_secs(config.short_break_minutes * 60),
            Self::LongBreak => Duration::from_secs(config.long_break_minutes * 60),
        }
    }

    pub fn label(self) -> &'static str {
        self.label_with(crate::i18n::Lang::default())
    }

    pub fn label_with(self, lang: crate::i18n::Lang) -> &'static str {
        let s = crate::i18n::strings(lang);
        match self {
            Self::Work => s.work,
            Self::ShortBreak => s.short_break,
            Self::LongBreak => s.long_break,
        }
    }

    pub fn is_break(self) -> bool {
        matches!(self, Self::ShortBreak | Self::LongBreak)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimerStatus {
    Idle,
    Running,
    Paused,
    Finished,
}

pub struct Timer {
    pub phase: TimerPhase,
    pub status: TimerStatus,
    pub completed_sessions: u32,
    pub current_task: Option<String>,
    pub config: TimerConfig,
    pub phase_started_at: Option<DateTime<Local>>,
    pub skipped_breaks: u32,
    total_duration: Duration,
    elapsed: Duration,
    last_tick: Option<Instant>,
}

impl Default for Timer {
    fn default() -> Self {
        Self::new()
    }
}

impl Timer {
    pub fn new() -> Self {
        let config = TimerConfig::default();
        let phase = TimerPhase::Work;
        let total_duration = phase.duration(&config);
        Self {
            phase,
            status: TimerStatus::Idle,
            completed_sessions: 0,
            current_task: None,
            config,
            phase_started_at: None,
            skipped_breaks: 0,
            total_duration,
            elapsed: Duration::ZERO,
            last_tick: None,
        }
    }

    pub fn start(&mut self) {
        match self.status {
            TimerStatus::Idle | TimerStatus::Paused | TimerStatus::Finished => {
                if self.status == TimerStatus::Finished {
                    self.advance_phase();
                    self.elapsed = Duration::ZERO;
                }
                if self.phase_started_at.is_none() {
                    self.phase_started_at = Some(Local::now());
                }
                self.status = TimerStatus::Running;
                self.last_tick = Some(Instant::now());
            }
            TimerStatus::Running => {}
        }
    }

    pub fn pause(&mut self) {
        if self.status == TimerStatus::Running {
            self.accumulate_elapsed();
            self.status = TimerStatus::Paused;
            self.last_tick = None;
        }
    }

    pub fn reset(&mut self) {
        self.status = TimerStatus::Idle;
        self.elapsed = Duration::ZERO;
        self.total_duration = self.phase.duration(&self.config);
        self.last_tick = None;
        self.phase_started_at = None;
    }

    pub fn skip(&mut self) {
        if self.phase == TimerPhase::Work {
            self.completed_sessions += 1;
        }
        // Track skipped breaks for overwork warning
        if self.phase.is_break() {
            // Break was skipped (not completed naturally)
            self.skipped_breaks += 1;
        } else {
            self.skipped_breaks = 0;
        }
        self.advance_phase();
        self.status = TimerStatus::Idle;
        self.elapsed = Duration::ZERO;
        self.last_tick = None;
        self.phase_started_at = None;
    }

    /// End the current session early. Returns (completed_phase, actual_minutes).
    /// Records actual elapsed time instead of the configured duration.
    pub fn done(&mut self) -> Option<(TimerPhase, u64)> {
        if !matches!(self.status, TimerStatus::Running | TimerStatus::Paused) {
            return None;
        }

        self.accumulate_elapsed();
        let actual_secs = self.current_elapsed().as_secs();
        let actual_minutes = ((actual_secs + 30) / 60).max(1);

        let finished_phase = self.phase;

        if finished_phase == TimerPhase::Work {
            self.completed_sessions += 1;
        }
        if finished_phase.is_break() {
            self.skipped_breaks = 0;
        }

        self.advance_phase();
        self.status = TimerStatus::Idle;
        self.elapsed = Duration::ZERO;
        self.last_tick = None;
        self.phase_started_at = None;

        Some((finished_phase, actual_minutes))
    }

    /// Called every frame. Returns `Some(completed_phase)` when a phase finishes.
    pub fn tick(&mut self) -> Option<TimerPhase> {
        if self.status != TimerStatus::Running {
            return None;
        }

        self.accumulate_elapsed();
        self.last_tick = Some(Instant::now());

        if self.elapsed >= self.total_duration {
            self.elapsed = self.total_duration;
            self.status = TimerStatus::Finished;
            self.last_tick = None;
            let finished_phase = self.phase;
            if finished_phase == TimerPhase::Work {
                self.completed_sessions += 1;
            }
            // Break completed naturally → reset skip counter
            if finished_phase.is_break() {
                self.skipped_breaks = 0;
            }
            return Some(finished_phase);
        }

        None
    }

    /// Check if auto-start should trigger for the completed phase.
    pub fn should_auto_start(&self, completed_phase: TimerPhase) -> bool {
        match completed_phase {
            // Work just finished → next is break → auto_start_break?
            TimerPhase::Work => self.config.auto_start_break,
            // Break just finished → next is work → auto_start_work?
            TimerPhase::ShortBreak | TimerPhase::LongBreak => self.config.auto_start_work,
        }
    }

    /// Progress from 0.0 (just started) to 1.0 (complete).
    pub fn progress(&self) -> f32 {
        if self.total_duration.is_zero() {
            return 1.0;
        }
        let elapsed_secs = self.current_elapsed().as_secs_f32();
        let total_secs = self.total_duration.as_secs_f32();
        (elapsed_secs / total_secs).clamp(0.0, 1.0)
    }

    /// Remaining time formatted as "MM:SS".
    pub fn remaining_display(&self) -> String {
        let remaining = self.total_duration.saturating_sub(self.current_elapsed());
        let total_secs = remaining.as_secs();
        let minutes = total_secs / 60;
        let seconds = total_secs % 60;
        format!("{minutes:02}:{seconds:02}")
    }

    /// Apply new config. Takes effect on next phase (not current running phase).
    pub fn set_config(&mut self, config: TimerConfig) {
        self.config = config;
        if self.status == TimerStatus::Idle {
            self.total_duration = self.phase.duration(&self.config);
        }
    }

    fn current_elapsed(&self) -> Duration {
        let mut elapsed = self.elapsed;
        if let Some(last) = self.last_tick {
            elapsed += last.elapsed();
        }
        elapsed.min(self.total_duration)
    }

    fn accumulate_elapsed(&mut self) {
        if let Some(last) = self.last_tick {
            self.elapsed += last.elapsed();
        }
    }

    fn advance_phase(&mut self) {
        self.phase = match self.phase {
            TimerPhase::Work => {
                if self.completed_sessions >= SESSIONS_BEFORE_LONG_BREAK {
                    TimerPhase::LongBreak
                } else {
                    TimerPhase::ShortBreak
                }
            }
            TimerPhase::ShortBreak | TimerPhase::LongBreak => {
                if self.phase == TimerPhase::LongBreak {
                    self.completed_sessions = 0;
                }
                TimerPhase::Work
            }
        };
        self.total_duration = self.phase.duration(&self.config);
        self.phase_started_at = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn initial_state() {
        let timer = Timer::new();
        assert_eq!(timer.phase, TimerPhase::Work);
        assert_eq!(timer.status, TimerStatus::Idle);
        assert_eq!(timer.completed_sessions, 0);
        assert!(timer.current_task.is_none());
        assert_eq!(timer.skipped_breaks, 0);
    }

    #[test]
    fn start_sets_running() {
        let mut timer = Timer::new();
        timer.start();
        assert_eq!(timer.status, TimerStatus::Running);
        assert!(timer.phase_started_at.is_some());
    }

    #[test]
    fn pause_and_resume() {
        let mut timer = Timer::new();
        timer.start();
        timer.pause();
        assert_eq!(timer.status, TimerStatus::Paused);
        timer.start();
        assert_eq!(timer.status, TimerStatus::Running);
    }

    #[test]
    fn reset_returns_to_idle() {
        let mut timer = Timer::new();
        timer.start();
        thread::sleep(Duration::from_millis(10));
        timer.reset();
        assert_eq!(timer.status, TimerStatus::Idle);
        assert_eq!(timer.remaining_display(), "25:00");
        assert!(timer.phase_started_at.is_none());
    }

    #[test]
    fn skip_advances_phase() {
        let mut timer = Timer::new();
        assert_eq!(timer.phase, TimerPhase::Work);
        timer.skip();
        assert_eq!(timer.phase, TimerPhase::ShortBreak);
        assert_eq!(timer.status, TimerStatus::Idle);
        assert_eq!(timer.completed_sessions, 1);
    }

    #[test]
    fn four_work_sessions_trigger_long_break() {
        let mut timer = Timer::new();
        for i in 0..4 {
            assert_eq!(timer.phase, TimerPhase::Work, "iteration {i}");
            timer.skip();
            if i < 3 {
                assert_eq!(timer.phase, TimerPhase::ShortBreak, "iteration {i}");
                timer.skip();
            }
        }
        assert_eq!(timer.phase, TimerPhase::LongBreak);
    }

    #[test]
    fn long_break_resets_sessions() {
        let mut timer = Timer::new();
        for _ in 0..3 {
            timer.skip();
            timer.skip();
        }
        timer.skip();
        assert_eq!(timer.phase, TimerPhase::LongBreak);
        timer.skip();
        assert_eq!(timer.phase, TimerPhase::Work);
        assert_eq!(timer.completed_sessions, 0);
    }

    #[test]
    fn remaining_display_format() {
        let timer = Timer::new();
        assert_eq!(timer.remaining_display(), "25:00");
    }

    #[test]
    fn remaining_display_short_break() {
        let mut timer = Timer::new();
        timer.skip();
        assert_eq!(timer.remaining_display(), "05:00");
    }

    #[test]
    fn progress_starts_at_zero() {
        let timer = Timer::new();
        assert!((timer.progress()).abs() < 0.01);
    }

    #[test]
    fn tick_returns_none_when_idle() {
        let mut timer = Timer::new();
        assert!(timer.tick().is_none());
    }

    #[test]
    fn tick_returns_none_when_paused() {
        let mut timer = Timer::new();
        timer.start();
        timer.pause();
        assert!(timer.tick().is_none());
    }

    #[test]
    fn phase_durations_with_config() {
        let config = TimerConfig::default();
        assert_eq!(
            TimerPhase::Work.duration(&config),
            Duration::from_secs(25 * 60)
        );
        assert_eq!(
            TimerPhase::ShortBreak.duration(&config),
            Duration::from_secs(5 * 60)
        );
        assert_eq!(
            TimerPhase::LongBreak.duration(&config),
            Duration::from_secs(15 * 60)
        );
    }

    #[test]
    fn custom_config() {
        let config = TimerConfig {
            work_minutes: 50,
            short_break_minutes: 10,
            long_break_minutes: 30,
            ..TimerConfig::default()
        };
        let mut timer = Timer::new();
        timer.set_config(config);
        assert_eq!(timer.remaining_display(), "50:00");
        timer.skip();
        assert_eq!(timer.remaining_display(), "10:00");
    }

    #[test]
    fn set_config_during_idle_updates_duration() {
        let mut timer = Timer::new();
        assert_eq!(timer.remaining_display(), "25:00");
        timer.set_config(TimerConfig {
            work_minutes: 45,
            ..TimerConfig::default()
        });
        assert_eq!(timer.remaining_display(), "45:00");
    }

    #[test]
    fn task_name() {
        let mut timer = Timer::new();
        timer.current_task = Some("Write report".to_string());
        assert_eq!(timer.current_task.as_deref(), Some("Write report"));
        timer.current_task = None;
        assert!(timer.current_task.is_none());
    }

    #[test]
    fn phase_labels() {
        assert_eq!(TimerPhase::Work.label(), "集中");
        assert_eq!(TimerPhase::ShortBreak.label(), "休憩");
        assert_eq!(TimerPhase::LongBreak.label(), "長休憩");
    }

    #[test]
    fn skipped_breaks_tracking() {
        let mut timer = Timer::new();
        timer.skip(); // Work → ShortBreak (skipped_breaks stays 0, was work)
        assert_eq!(timer.skipped_breaks, 0);
        timer.skip(); // ShortBreak → Work (break skipped → 1)
        assert_eq!(timer.skipped_breaks, 1);
        timer.skip(); // Work → ShortBreak (work skipped → resets to 0)
        assert_eq!(timer.skipped_breaks, 0);
    }

    #[test]
    fn done_while_idle_returns_none() {
        let mut timer = Timer::new();
        assert!(timer.done().is_none());
    }

    #[test]
    fn done_while_running_returns_phase_and_advances() {
        let mut timer = Timer::new();
        timer.start();
        thread::sleep(Duration::from_millis(80));
        let result = timer.done();
        assert!(result.is_some());
        let (phase, minutes) = result.unwrap();
        assert_eq!(phase, TimerPhase::Work);
        assert_eq!(minutes, 1); // at least 1 minute
        assert_eq!(timer.phase, TimerPhase::ShortBreak);
        assert_eq!(timer.status, TimerStatus::Idle);
        assert_eq!(timer.completed_sessions, 1);
    }

    #[test]
    fn done_while_paused() {
        let mut timer = Timer::new();
        timer.start();
        thread::sleep(Duration::from_millis(50));
        timer.pause();
        let result = timer.done();
        assert!(result.is_some());
        let (phase, _) = result.unwrap();
        assert_eq!(phase, TimerPhase::Work);
        assert_eq!(timer.status, TimerStatus::Idle);
    }

    #[test]
    fn auto_start_config() {
        let mut timer = Timer::new();
        timer.config.auto_start_work = true;
        timer.config.auto_start_break = true;
        assert!(timer.should_auto_start(TimerPhase::Work)); // work done → auto break
        assert!(timer.should_auto_start(TimerPhase::ShortBreak)); // break done → auto work
    }
}
