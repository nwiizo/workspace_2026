use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Lang {
    #[default]
    Ja,
    En,
}

impl Lang {
    pub fn toggle(self) -> Self {
        match self {
            Self::Ja => Self::En,
            Self::En => Self::Ja,
        }
    }
}

pub struct Strings {
    // Timer phase labels
    pub work: &'static str,
    pub short_break: &'static str,
    pub long_break: &'static str,

    // Notification: phase complete
    pub notify_work_done_summary: &'static str,
    pub notify_work_done_body: &'static str,
    pub notify_short_break_done_summary: &'static str,
    pub notify_short_break_done_body: &'static str,
    pub notify_long_break_done_summary: &'static str,
    pub notify_long_break_done_body: &'static str,

    // Notification: overwork escalation
    pub overwork_2_summary: &'static str,
    pub overwork_2_body: &'static str,
    pub overwork_3_summary: &'static str,
    pub overwork_3_body: &'static str,
    pub overwork_4_summary: &'static str,
    pub overwork_4_body: &'static str,

    // Notification: late night
    pub late_22_summary: &'static str,
    pub late_22_body: &'static str,
    pub late_23_summary: &'static str,
    pub late_23_body: &'static str,
    pub late_0_summary: &'static str,
    pub late_0_body: &'static str,

    // UI
    pub task_hint: &'static str,
    pub stats_format: &'static str, // "{pomodoros} / {minutes}"
    pub report_today: &'static str,
    pub report_week: &'static str,
    pub today_log: &'static str,
    pub min_short: &'static str,
    pub no_records: &'static str,

    // CLI
    pub status_idle: &'static str,
    pub status_running: &'static str,
    pub status_paused: &'static str,
    pub status_finished: &'static str,
    pub cli_error_not_running: &'static str,
    pub cli_no_history: &'static str,

    // IPC responses
    pub resp_start: &'static str,
    pub resp_pause: &'static str,
    pub resp_reset: &'static str,
    pub resp_skip: &'static str,
    pub resp_quit: &'static str,
    pub resp_done: &'static str,
    pub resp_log: &'static str,
    pub resp_task_clear: &'static str,
}

const JA: Strings = Strings {
    work: "集中",
    short_break: "休憩",
    long_break: "長休憩",

    notify_work_done_summary: "おつかれ。",
    notify_work_done_body: "休め。",
    notify_short_break_done_summary: "さて。",
    notify_short_break_done_body: "やるか。",
    notify_long_break_done_summary: "充電完了。",
    notify_long_break_done_body: "いこう。",

    overwork_2_summary: "おい、休め。",
    overwork_2_body: "2回続けて休憩を飛ばした。",
    overwork_3_summary: "まだやるのか。",
    overwork_3_body: "身体は一つしかない。",
    overwork_4_summary: "いい加減にしろ。",
    overwork_4_body: "画面を閉じて水を飲め。",

    late_22_summary: "そろそろいい時間だ。",
    late_22_body: "明日の自分に任せろ。",
    late_23_summary: "おい、寝ろ。",
    late_23_body: "今日はもう充分やった。",
    late_0_summary: "何時だと思ってる。",
    late_0_body: "寝ろ。これは命令だ。",

    task_hint: "何やる？",
    stats_format: "今日: {pomodoros}回 / {minutes}分",
    report_today: "今日",
    report_week: "今週",
    today_log: "今日の記録",
    min_short: "分",
    no_records: "記録なし",

    status_idle: "待機",
    status_running: "実行中",
    status_paused: "一時停止",
    status_finished: "完了",
    cli_error_not_running: "タイマーが起動していますか？ → yasume",
    cli_no_history: "履歴がありません。",

    resp_start: "開始",
    resp_pause: "一時停止",
    resp_reset: "リセット",
    resp_skip: "スキップ",
    resp_quit: "終了",
    resp_done: "途中完了",
    resp_log: "記録追加",
    resp_task_clear: "タスク解除",
};

const EN: Strings = Strings {
    work: "Focus",
    short_break: "Break",
    long_break: "Long Break",

    notify_work_done_summary: "Good work.",
    notify_work_done_body: "Take a break.",
    notify_short_break_done_summary: "Alright.",
    notify_short_break_done_body: "Let's go.",
    notify_long_break_done_summary: "Recharged.",
    notify_long_break_done_body: "Ready.",

    overwork_2_summary: "Hey, take a break.",
    overwork_2_body: "You skipped 2 breaks in a row.",
    overwork_3_summary: "Still going?",
    overwork_3_body: "You only have one body.",
    overwork_4_summary: "Enough.",
    overwork_4_body: "Close the screen and drink water.",

    late_22_summary: "It's getting late.",
    late_22_body: "Leave it to tomorrow's you.",
    late_23_summary: "Hey, sleep.",
    late_23_body: "You've done enough today.",
    late_0_summary: "Do you know what time it is?",
    late_0_body: "Sleep. That's an order.",

    task_hint: "What's up?",
    stats_format: "Today: {pomodoros} / {minutes}min",
    report_today: "Today",
    report_week: "This Week",
    today_log: "Today's Log",
    min_short: "min",
    no_records: "No records",

    status_idle: "Idle",
    status_running: "Running",
    status_paused: "Paused",
    status_finished: "Finished",
    cli_error_not_running: "Is the timer running? → yasume",
    cli_no_history: "No history.",

    resp_start: "Started",
    resp_pause: "Paused",
    resp_reset: "Reset",
    resp_skip: "Skipped",
    resp_quit: "Quit",
    resp_done: "Done early",
    resp_log: "Logged",
    resp_task_clear: "Task cleared",
};

pub fn strings(lang: Lang) -> &'static Strings {
    match lang {
        Lang::Ja => &JA,
        Lang::En => &EN,
    }
}

impl Strings {
    pub fn format_stats(&self, pomodoros: usize, minutes: u64) -> String {
        self.stats_format
            .replace("{pomodoros}", &pomodoros.to_string())
            .replace("{minutes}", &minutes.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggle_language() {
        assert_eq!(Lang::Ja.toggle(), Lang::En);
        assert_eq!(Lang::En.toggle(), Lang::Ja);
    }

    #[test]
    fn strings_ja() {
        let s = strings(Lang::Ja);
        assert_eq!(s.work, "集中");
        assert_eq!(s.short_break, "休憩");
    }

    #[test]
    fn strings_en() {
        let s = strings(Lang::En);
        assert_eq!(s.work, "Focus");
        assert_eq!(s.short_break, "Break");
    }

    #[test]
    fn format_stats_ja() {
        let s = strings(Lang::Ja);
        assert_eq!(s.format_stats(5, 125), "今日: 5回 / 125分");
    }

    #[test]
    fn format_stats_en() {
        let s = strings(Lang::En);
        assert_eq!(s.format_stats(5, 125), "Today: 5 / 125min");
    }

    #[test]
    fn default_lang_is_ja() {
        assert_eq!(Lang::default(), Lang::Ja);
    }
}
