//! rustc rejects this package (E0502). Relax type-checks each function before
//! it runs and borrow-checks in the background, so this package shows what a
//! run does before and after the borrow checker reports `bad`.
//!
//! `main` prints, waits, and only then calls `bad` when `CALL_BAD=1` is set.

use std::time::Duration;

fn bad() -> i32 {
    let mut v = vec![1, 2, 3];
    let first = &v[0];
    v.push(4);
    *first
}

fn main() {
    println!("main started");
    let call_bad = std::env::var("CALL_BAD").is_ok_and(|v| v == "1");
    let wait_ms: u64 = std::env::var("WAIT_MS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    std::thread::sleep(Duration::from_millis(wait_ms));
    if call_bad {
        println!("calling bad()");
        println!("bad() = {}", bad());
    }
    println!("main finished");
}
