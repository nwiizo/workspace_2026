//! One CPU-bound workload repeated for several rounds. Each round prints its
//! own time, so the log shows Relax's tier-up (interpreter -> O1 -> O3) round
//! by round next to a `cargo run --release` build of the same source.
//! The checksum must match between the two toolchains.

use std::time::Instant;

fn xorshift(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}

fn sort_work(n: usize, seed: u64) -> u64 {
    let mut s = seed;
    let mut v: Vec<u64> = (0..n).map(|_| xorshift(&mut s)).collect();
    v.sort_unstable();
    v.iter()
        .step_by(1024)
        .fold(0u64, |acc, x| acc.wrapping_add(*x))
}

fn sieve_work(limit: usize) -> u64 {
    let mut composite = vec![false; limit + 1];
    let mut sum = 0u64;
    for i in 2..=limit {
        if !composite[i] {
            sum = sum.wrapping_add(i as u64);
            let mut j = i * i;
            while j <= limit {
                composite[j] = true;
                j += i;
            }
        }
    }
    sum
}

fn main() {
    let rounds: usize = std::env::var("ROUNDS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(8);
    let mut checksum = 0u64;
    for round in 1..=rounds {
        let t = Instant::now();
        checksum = checksum
            .wrapping_add(sort_work(2_000_000, 0x9E37_79B9_7F4A_7C15 ^ round as u64))
            .wrapping_add(sieve_work(10_000_000));
        println!("round {round}: {} ms", t.elapsed().as_millis());
    }
    println!("checksum {checksum}");
}
