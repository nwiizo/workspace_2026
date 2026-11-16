//! A package with crates.io dependencies and a procedural macro (serde derive),
//! to compare a cold start under Relax with a cold `cargo run`.

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct State {
    counter: i64,
    message: String,
    tags: Vec<String>,
}

fn main() {
    let state = State {
        counter: 42,
        message: "hello from serde".into(),
        tags: vec!["relax".into(), "rustc".into()],
    };
    let json = match serde_json::to_string(&state) {
        Ok(json) => json,
        Err(e) => {
            eprintln!("serialize failed: {e}");
            std::process::exit(1);
        }
    };
    println!("{json}");
    let back: State = match serde_json::from_str(&json) {
        Ok(back) => back,
        Err(e) => {
            eprintln!("deserialize failed: {e}");
            std::process::exit(1);
        }
    };
    println!("roundtrip ok: {}", back == state);
}
