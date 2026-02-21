use std::process::Command as ProcessCommand;

use chrono::{Local, Timelike};

use crate::i18n::{Lang, strings};
use crate::timer::TimerPhase;

pub fn notify_phase_complete(phase: TimerPhase, task_name: Option<&str>, lang: Lang) {
    let s = strings(lang);
    let (summary, body_str) = match phase {
        TimerPhase::Work => (s.notify_work_done_summary, s.notify_work_done_body),
        TimerPhase::ShortBreak => (
            s.notify_short_break_done_summary,
            s.notify_short_break_done_body,
        ),
        TimerPhase::LongBreak => (
            s.notify_long_break_done_summary,
            s.notify_long_break_done_body,
        ),
    };

    let body = match task_name {
        Some(name) => format!("{body_str} — {name}"),
        None => body_str.to_string(),
    };

    if let Err(e) = notify_rust::Notification::new()
        .summary(summary)
        .body(&body)
        .sound_name("Glass")
        .show()
    {
        eprintln!("Notification failed: {e}");
    }

    play_sound(phase);
}

pub fn notify_overwork(skipped_count: u32, lang: Lang) {
    let s = strings(lang);
    let (summary, body) = match skipped_count {
        2 => (s.overwork_2_summary, s.overwork_2_body),
        3 => (s.overwork_3_summary, s.overwork_3_body),
        _ => (s.overwork_4_summary, s.overwork_4_body),
    };

    if let Err(e) = notify_rust::Notification::new()
        .summary(summary)
        .body(body)
        .sound_name("Sosumi")
        .show()
    {
        eprintln!("Notification failed: {e}");
    }
    let _ = ProcessCommand::new("afplay")
        .arg("/System/Library/Sounds/Sosumi.aiff")
        .spawn();
}

pub fn notify_late_night(lang: Lang) {
    let hour = Local::now().hour();
    let s = strings(lang);
    let (summary, body) = match hour {
        22 => (s.late_22_summary, s.late_22_body),
        23 => (s.late_23_summary, s.late_23_body),
        0..=4 => (s.late_0_summary, s.late_0_body),
        _ => return,
    };

    if let Err(e) = notify_rust::Notification::new()
        .summary(summary)
        .body(body)
        .sound_name("Sosumi")
        .show()
    {
        eprintln!("Notification failed: {e}");
    }
}

/// Returns true if it's late night (22:00+)
pub fn is_late_night() -> bool {
    let hour = Local::now().hour();
    !(5..22).contains(&hour)
}

fn play_sound(phase: TimerPhase) {
    let sound = match phase {
        TimerPhase::Work => "/System/Library/Sounds/Glass.aiff",
        TimerPhase::ShortBreak | TimerPhase::LongBreak => "/System/Library/Sounds/Tink.aiff",
    };
    let _ = ProcessCommand::new("afplay").arg(sound).spawn();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn late_night_detection() {
        let _ = is_late_night();
    }
}
