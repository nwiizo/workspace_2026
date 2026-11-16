//! The same State / step / render shape as ../hotreload-rs/part1-hot-lib-reloader,
//! written as one plain binary crate. Under Relax there is no dylib and no
//! reloader crate: edit `step` while `relax run live-counter` is running.
//!
//! Each line starts with the wall clock in milliseconds since the Unix epoch,
//! so the time from a save (the file's mtime) to the first changed line can be
//! read from the log.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub struct State {
    pub counter: i64,
    pub message: String,
}

impl State {
    pub fn new() -> Self {
        Self {
            counter: 0,
            message: String::from("start"),
        }
    }
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}

pub fn step(state: &mut State) {
    state.counter += 1;
    state.message = format!("counter = {}", state.counter);
}

pub fn render(state: &State) -> String {
    format!("[live] {}", state.message)
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

fn main() {
    let mut state = State::new();
    let mut iteration: u64 = 0;
    loop {
        iteration += 1;
        step(&mut state);
        println!("{} iter={} {}", now_ms(), iteration, render(&state));
        std::thread::sleep(Duration::from_millis(50));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn step_increments_counter() {
        let mut state = State::new();
        step(&mut state);
        step(&mut state);
        assert_eq!(state.counter, 2);
    }

    #[test]
    fn render_contains_message() {
        let state = State {
            counter: 42,
            message: "test".into(),
        };
        assert!(render(&state).contains("test"));
    }
}
