use chrono::{DateTime, Datelike, Local, NaiveDate};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;

use crate::timer::TimerPhase;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletedTask {
    pub task_name: Option<String>,
    pub phase: TimerPhase,
    pub started_at: DateTime<Local>,
    pub completed_at: DateTime<Local>,
    pub duration_minutes: u64,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct History {
    pub tasks: Vec<CompletedTask>,
}

/// Today's stats: (pomodoro_count, total_focus_minutes, unique_task_count)
pub struct TodayStats {
    pub pomodoros: usize,
    pub focus_minutes: u64,
    pub task_count: usize,
}

fn history_path() -> Option<PathBuf> {
    dirs::data_local_dir().map(|d| d.join("yasume").join("history.json"))
}

impl History {
    pub fn load() -> Self {
        let Some(path) = history_path() else {
            return Self::default();
        };
        if !path.exists() {
            return Self::default();
        }
        match std::fs::read_to_string(&path) {
            Ok(data) => serde_json::from_str(&data).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self) {
        let Some(path) = history_path() else {
            return;
        };
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        match serde_json::to_string_pretty(&self) {
            Ok(json) => {
                let _ = std::fs::write(&path, json);
            }
            Err(e) => eprintln!("Failed to save history: {e}"),
        }
    }

    pub fn record(
        &mut self,
        phase: TimerPhase,
        task_name: Option<String>,
        started_at: Option<DateTime<Local>>,
        duration_minutes: u64,
    ) {
        let now = Local::now();
        self.tasks.push(CompletedTask {
            task_name,
            phase,
            started_at: started_at.unwrap_or(now),
            completed_at: now,
            duration_minutes,
        });
        self.save();
    }

    pub fn today(&self) -> Vec<&CompletedTask> {
        let today = Local::now().date_naive();
        self.tasks
            .iter()
            .filter(|t| t.completed_at.date_naive() == today)
            .collect()
    }

    pub fn today_stats(&self) -> TodayStats {
        let today_tasks = self.today();
        let pomodoros = today_tasks
            .iter()
            .filter(|t| t.phase == TimerPhase::Work)
            .count();
        let focus_minutes: u64 = today_tasks
            .iter()
            .filter(|t| t.phase == TimerPhase::Work)
            .map(|t| t.duration_minutes)
            .sum();
        let task_count = today_tasks
            .iter()
            .filter_map(|t| t.task_name.as_ref())
            .collect::<HashSet<_>>()
            .len();
        TodayStats {
            pomodoros,
            focus_minutes,
            task_count,
        }
    }

    pub fn this_week(&self) -> Vec<&CompletedTask> {
        let today = Local::now().date_naive();
        let days_from_monday = today.weekday().num_days_from_monday();
        let week_start = today - chrono::Duration::days(i64::from(days_from_monday));
        self.tasks
            .iter()
            .filter(|t| t.completed_at.date_naive() >= week_start)
            .collect()
    }

    /// Returns unique work task names from the past N days, ordered by recency (newest first).
    pub fn recent_task_names(&self, days: i64) -> Vec<String> {
        let cutoff = Local::now().date_naive() - chrono::Duration::days(days);
        let mut seen = HashSet::new();
        let mut names = Vec::new();
        for task in self.tasks.iter().rev() {
            if task.completed_at.date_naive() < cutoff {
                continue;
            }
            if task.phase != TimerPhase::Work {
                continue;
            }
            if let Some(name) = &task.task_name
                && seen.insert(name.clone())
            {
                names.push(name.clone());
            }
        }
        names
    }

    pub fn filter_by_date_range(
        &self,
        from: Option<NaiveDate>,
        to: Option<NaiveDate>,
    ) -> Vec<&CompletedTask> {
        let from = from.unwrap_or_else(|| Local::now().date_naive());
        let to = to.unwrap_or(from);
        self.tasks
            .iter()
            .filter(|t| {
                let date = t.completed_at.date_naive();
                date >= from && date <= to
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_and_retrieve() {
        let mut history = History::default();
        history.tasks.push(CompletedTask {
            task_name: Some("Test task".to_string()),
            phase: TimerPhase::Work,
            started_at: Local::now(),
            completed_at: Local::now(),
            duration_minutes: 25,
        });
        assert_eq!(history.tasks.len(), 1);
        assert_eq!(history.tasks[0].task_name.as_deref(), Some("Test task"));
        assert_eq!(history.tasks[0].phase, TimerPhase::Work);
        assert_eq!(history.tasks[0].duration_minutes, 25);
    }

    #[test]
    fn today_filter() {
        let mut history = History::default();
        let now = Local::now();
        history.tasks.push(CompletedTask {
            task_name: None,
            phase: TimerPhase::Work,
            started_at: now,
            completed_at: now,
            duration_minutes: 25,
        });
        history.tasks.push(CompletedTask {
            task_name: None,
            phase: TimerPhase::ShortBreak,
            started_at: now,
            completed_at: now,
            duration_minutes: 5,
        });
        assert_eq!(history.today().len(), 2);
    }

    #[test]
    fn today_stats_calculation() {
        let mut history = History::default();
        let now = Local::now();
        for _ in 0..3 {
            history.tasks.push(CompletedTask {
                task_name: Some("Task A".to_string()),
                phase: TimerPhase::Work,
                started_at: now,
                completed_at: now,
                duration_minutes: 25,
            });
        }
        history.tasks.push(CompletedTask {
            task_name: Some("Task B".to_string()),
            phase: TimerPhase::Work,
            started_at: now,
            completed_at: now,
            duration_minutes: 25,
        });
        history.tasks.push(CompletedTask {
            task_name: None,
            phase: TimerPhase::ShortBreak,
            started_at: now,
            completed_at: now,
            duration_minutes: 5,
        });
        let stats = history.today_stats();
        assert_eq!(stats.pomodoros, 4);
        assert_eq!(stats.focus_minutes, 100);
        assert_eq!(stats.task_count, 2); // "Task A" and "Task B"
    }

    #[test]
    fn empty_history() {
        let history = History::default();
        assert!(history.today().is_empty());
        assert!(history.tasks.is_empty());
        let stats = history.today_stats();
        assert_eq!(stats.pomodoros, 0);
        assert_eq!(stats.focus_minutes, 0);
    }

    #[test]
    fn filter_by_date_range_today_default() {
        let mut history = History::default();
        let now = Local::now();
        history.tasks.push(CompletedTask {
            task_name: Some("Today task".to_string()),
            phase: TimerPhase::Work,
            started_at: now,
            completed_at: now,
            duration_minutes: 25,
        });
        let result = history.filter_by_date_range(None, None);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn recent_task_names_ordered_by_recency() {
        let mut history = History::default();
        let now = Local::now();
        // Add tasks in order: A, B, A, C
        for name in &["A", "B", "A", "C"] {
            history.tasks.push(CompletedTask {
                task_name: Some(name.to_string()),
                phase: TimerPhase::Work,
                started_at: now,
                completed_at: now,
                duration_minutes: 25,
            });
        }
        // Break tasks should be excluded
        history.tasks.push(CompletedTask {
            task_name: Some("BreakTask".to_string()),
            phase: TimerPhase::ShortBreak,
            started_at: now,
            completed_at: now,
            duration_minutes: 5,
        });
        let names = history.recent_task_names(7);
        // Newest first, unique: C, A, B
        assert_eq!(names, vec!["C", "A", "B"]);
    }

    #[test]
    fn recent_task_names_empty() {
        let history = History::default();
        assert!(history.recent_task_names(7).is_empty());
    }

    #[test]
    fn filter_by_date_range_specific() {
        let mut history = History::default();
        let now = Local::now();
        let today = now.date_naive();
        history.tasks.push(CompletedTask {
            task_name: Some("Task".to_string()),
            phase: TimerPhase::Work,
            started_at: now,
            completed_at: now,
            duration_minutes: 25,
        });
        // Same day should match
        let result = history.filter_by_date_range(Some(today), Some(today));
        assert_eq!(result.len(), 1);
        // Future date should not match
        let future = today + chrono::Duration::days(1);
        let result = history.filter_by_date_range(Some(future), Some(future));
        assert!(result.is_empty());
    }
}
