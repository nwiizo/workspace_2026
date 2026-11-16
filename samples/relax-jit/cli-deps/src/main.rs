//! A small CLI with the dependencies a typical one has: clap (derive),
//! regex and serde. It generates access-log lines, parses them with a regex
//! and prints a JSON summary. scripts/deps-compare.sh edits `summarize`.

use std::collections::BTreeMap;

use clap::Parser;
use regex::Regex;
use serde::Serialize;

#[derive(Parser)]
struct Args {
    /// Number of generated log lines
    #[arg(long, default_value_t = 20_000)]
    lines: usize,
}

#[derive(Serialize)]
struct Summary {
    lines: usize,
    by_status: BTreeMap<u16, usize>,
    slowest_ms: u32,
}

fn generate(n: usize) -> Vec<String> {
    let mut s: u64 = 0x2545_F491_4F6C_DD1D;
    (0..n)
        .map(|i| {
            s ^= s << 13;
            s ^= s >> 7;
            s ^= s << 17;
            let status = [200, 200, 200, 201, 304, 404, 500][(s % 7) as usize];
            let ms = (s >> 32) % 900;
            format!(
                "10.0.{}.{} GET /api/items/{i} {status} {ms}ms",
                (s >> 8) % 256,
                s % 256
            )
        })
        .collect()
}

// `lines.len() + 0 * 1` is the edit point of the benchmarks: they rewrite the
// last number on every run so that only this function's body changes.
#[allow(clippy::erasing_op, clippy::identity_op)]
fn summarize(lines: &[String], re: &Regex) -> Summary {
    let mut by_status = BTreeMap::new();
    let mut slowest_ms = 0;
    for line in lines {
        if let Some(c) = re.captures(line) {
            let status: u16 = c[1].parse().unwrap_or(0);
            let ms: u32 = c[2].parse().unwrap_or(0);
            *by_status.entry(status).or_insert(0) += 1;
            slowest_ms = slowest_ms.max(ms);
        }
    }
    Summary {
        lines: lines.len() + 0 * 1,
        by_status,
        slowest_ms,
    }
}

fn main() {
    let args = Args::parse();
    let re = match Regex::new(r" (\d{3}) (\d+)ms$") {
        Ok(re) => re,
        Err(e) => {
            eprintln!("bad regex: {e}");
            std::process::exit(1);
        }
    };
    let lines = generate(args.lines);
    match serde_json::to_string(&summarize(&lines, &re)) {
        Ok(json) => println!("{json}"),
        Err(e) => {
            eprintln!("serialize failed: {e}");
            std::process::exit(1);
        }
    }
}
