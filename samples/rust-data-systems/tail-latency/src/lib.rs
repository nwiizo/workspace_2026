/// Compute the theoretical probability that at least one of N
/// independent requests hits the tail (above the p-th percentile).
///
/// P(at least one slow) = 1 - (1 - p_tail)^N
pub fn theoretical_tail_probability(p_tail: f64, fan_out: u32) -> f64 {
    1.0 - (1.0 - p_tail).powi(fan_out as i32)
}

/// Compute percentile from a sorted slice of f64 values.
/// `p` is in [0, 100].
pub fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let rank = (p / 100.0) * (sorted.len() - 1) as f64;
    let lower = rank.floor() as usize;
    let upper = rank.ceil() as usize;
    let frac = rank - lower as f64;

    if lower == upper || upper >= sorted.len() {
        sorted[lower]
    } else {
        sorted[lower] * (1.0 - frac) + sorted[upper] * frac
    }
}

/// Tied requests: 同じrequestを2つのレプリカに即時送信し、
/// 「先に処理を開始した方」を採用、もう一方はキャンセル指示で破棄する。
/// 通信遅延 `cancel_signal_ms` だけ「他方が無駄に動く時間」がある。
/// `primary` と `backup` は同分布からの2サンプル、レイテンシは min。
/// 二重送信ぶんの追加負荷は約 (cancel_signal_ms / mean_latency) 倍に近似できる。
pub fn tied_request_latency(primary: f64, backup: f64, _cancel_signal_ms: f64) -> f64 {
    // 受信側からは「先に返ってきた方」が応答時間
    primary.min(backup)
}

/// 適応型ヘッジ: 直近 window 件のサンプルから動的に p95 を推定し、
/// それをヘッジ遅延として使う。負荷の変化に追随する Hedge のバリアント。
/// 戻り値は (実効レイテンシ, 推定 hedge_delay)。
pub fn adaptive_hedge_one_request(
    primary: f64,
    backup: f64,
    recent_samples: &[f64],
) -> (f64, f64) {
    if recent_samples.is_empty() {
        return (primary.min(backup), 0.0);
    }
    let mut sorted: Vec<f64> = recent_samples.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).expect("no NaN"));
    let hedge_delay = percentile(&sorted, 95.0);
    let effective = primary.min(hedge_delay + backup);
    (effective, hedge_delay)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::prelude::*;
    use rand_distr::LogNormal;

    fn generate_latencies(rng: &mut impl Rng, n: usize) -> Vec<f64> {
        // LogNormal with median ~5ms and heavy tail
        let dist = LogNormal::new(1.6_f64.ln(), 0.8).expect("valid distribution params");
        let mut latencies: Vec<f64> = (0..n).map(|_| rng.sample(dist)).collect();
        latencies.sort_by(|a, b| a.partial_cmp(b).expect("no NaN in latencies"));
        latencies
    }

    /// Simulate fan-out: each "user request" fans out to `fan_out` backends.
    /// The user-visible latency is max(all backend responses).
    fn simulate_fanout(rng: &mut impl Rng, fan_out: u32, num_requests: usize) -> Vec<f64> {
        let dist = LogNormal::new(1.6_f64.ln(), 0.8).expect("valid distribution params");
        let mut user_latencies = Vec::with_capacity(num_requests);

        for _ in 0..num_requests {
            let max_latency = (0..fan_out)
                .map(|_| rng.sample(dist))
                .fold(0.0_f64, f64::max);
            user_latencies.push(max_latency);
        }

        user_latencies.sort_by(|a, b| a.partial_cmp(b).expect("no NaN"));
        user_latencies
    }

    /// Simulate hedged requests: send to first backend, after `delay_ms`
    /// send duplicate to second backend, take whichever responds first.
    fn simulate_hedged(
        rng: &mut impl Rng,
        fan_out: u32,
        hedge_delay_percentile: f64,
        num_requests: usize,
    ) -> Vec<f64> {
        let dist = LogNormal::new(1.6_f64.ln(), 0.8).expect("valid distribution params");

        // Compute hedge delay from the base distribution
        let mut base_samples: Vec<f64> = (0..10_000).map(|_| rng.sample(dist)).collect();
        base_samples.sort_by(|a, b| a.partial_cmp(b).expect("no NaN"));
        let hedge_delay = percentile(&base_samples, hedge_delay_percentile);

        let mut user_latencies = Vec::with_capacity(num_requests);

        for _ in 0..num_requests {
            let max_latency = (0..fan_out)
                .map(|_| {
                    let primary: f64 = rng.sample(dist);
                    let backup: f64 = rng.sample(dist);
                    // Effective latency: min(primary, hedge_delay + backup)
                    primary.min(hedge_delay + backup)
                })
                .fold(0.0_f64, f64::max);
            user_latencies.push(max_latency);
        }

        user_latencies.sort_by(|a, b| a.partial_cmp(b).expect("no NaN"));
        user_latencies
    }

    #[test]
    fn theoretical_tail_probability_sanity() {
        // p99 with fan-out 1: 1%
        let p1 = theoretical_tail_probability(0.01, 1);
        assert!((p1 - 0.01).abs() < 0.001);

        // p99 with fan-out 10: ~9.6%
        let p10 = theoretical_tail_probability(0.01, 10);
        assert!((p10 - 0.0956).abs() < 0.01);

        // p99 with fan-out 100: ~63.4%
        let p100 = theoretical_tail_probability(0.01, 100);
        assert!((p100 - 0.634).abs() < 0.01);
    }

    #[test]
    fn percentile_calculation() {
        let data: Vec<f64> = (1..=100).map(|x| x as f64).collect();
        assert!((percentile(&data, 50.0) - 50.5).abs() < 0.1);
        assert!((percentile(&data, 99.0) - 99.01).abs() < 0.1);
        assert!((percentile(&data, 0.0) - 1.0).abs() < 0.01);
        assert!((percentile(&data, 100.0) - 100.0).abs() < 0.01);
    }

    #[test]
    fn fanout_amplifies_tail_latency() {
        let mut rng = StdRng::seed_from_u64(42);
        let num_requests = 100_000;

        let base = generate_latencies(&mut rng, num_requests);
        let base_p99 = percentile(&base, 99.0);
        let base_p50 = percentile(&base, 50.0);

        eprintln!("Base distribution: p50={base_p50:.2}ms, p99={base_p99:.2}ms");

        let mut prev_p99 = base_p99;
        for &fan_out in &[1, 2, 5, 10, 20] {
            let latencies = simulate_fanout(&mut rng, fan_out, num_requests);
            let p50 = percentile(&latencies, 50.0);
            let p99 = percentile(&latencies, 99.0);
            let p999 = percentile(&latencies, 99.9);

            eprintln!(
                "fan_out={fan_out:>2}: p50={p50:>7.2}ms, p99={p99:>7.2}ms, p99.9={p999:>7.2}ms"
            );

            if fan_out > 1 {
                assert!(
                    p99 > prev_p99 * 0.9,
                    "fan_out={fan_out}: p99 should generally increase"
                );
            }
            prev_p99 = p99;
        }
    }

    #[test]
    fn theoretical_vs_measured_tail_probability() {
        let mut rng = StdRng::seed_from_u64(123);
        let num_requests = 200_000;

        // Generate base latencies to find p99 threshold
        let base = generate_latencies(&mut rng, num_requests);
        let p99_threshold = percentile(&base, 99.0);

        for &fan_out in &[1, 5, 10, 20] {
            let latencies = simulate_fanout(&mut rng, fan_out, num_requests);

            // Count requests that exceed the base p99
            let exceeding = latencies.iter().filter(|&&l| l > p99_threshold).count();
            let measured = exceeding as f64 / num_requests as f64;
            let theoretical = theoretical_tail_probability(0.01, fan_out);

            eprintln!(
                "fan_out={fan_out:>2}: measured={measured:.4}, theoretical={theoretical:.4}, \
                 diff={:.4}",
                (measured - theoretical).abs()
            );

            // Allow reasonable tolerance (distribution shape affects exact match)
            // The theoretical formula assumes identical independent distributions
            assert!(
                measured > theoretical * 0.5,
                "fan_out={fan_out}: measured {measured} too far below theoretical {theoretical}"
            );
        }
    }

    #[test]
    fn hedged_requests_improve_tail() {
        let mut rng = StdRng::seed_from_u64(456);
        let num_requests = 100_000;
        let fan_out = 10;

        let normal = simulate_fanout(&mut rng, fan_out, num_requests);
        let hedged = simulate_hedged(&mut rng, fan_out, 95.0, num_requests);

        let normal_p99 = percentile(&normal, 99.0);
        let hedged_p99 = percentile(&hedged, 99.0);
        let improvement = (normal_p99 - hedged_p99) / normal_p99 * 100.0;

        eprintln!("Normal  p99: {normal_p99:.2}ms");
        eprintln!("Hedged  p99: {hedged_p99:.2}ms");
        eprintln!("Improvement: {improvement:.1}%");

        assert!(
            hedged_p99 < normal_p99,
            "hedged requests should reduce p99: {hedged_p99} >= {normal_p99}"
        );
    }

    #[test]
    fn fanout_p99_degradation_curve() {
        let mut rng = StdRng::seed_from_u64(789);
        let num_requests = 100_000;

        let base = generate_latencies(&mut rng, num_requests);
        let base_p99 = percentile(&base, 99.0);

        eprintln!("\nFan-out p99 degradation (base p99={base_p99:.2}ms):");
        eprintln!("{:<10} {:<12} {:<10}", "fan_out", "p99 (ms)", "ratio");
        eprintln!("{}", "-".repeat(32));

        for &fan_out in &[1, 2, 3, 5, 10, 20, 50] {
            let latencies = simulate_fanout(&mut rng, fan_out, num_requests);
            let p99 = percentile(&latencies, 99.0);
            let ratio = p99 / base_p99;
            eprintln!("{fan_out:<10} {p99:<12.2} {ratio:<10.2}x");
        }
    }

    #[test]
    fn tied_request_takes_minimum() {
        // 単純に min(primary, backup) と等価であることを確認
        let result = tied_request_latency(10.0, 3.0, 0.5);
        assert!((result - 3.0).abs() < f64::EPSILON);
        let result2 = tied_request_latency(1.0, 5.0, 0.5);
        assert!((result2 - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn adaptive_hedge_uses_recent_p95_as_delay() {
        // recent samples = 1..=100、p95 ≈ 95
        let samples: Vec<f64> = (1..=100).map(|x| x as f64).collect();
        let (effective, hedge_delay) = adaptive_hedge_one_request(200.0, 5.0, &samples);
        // primary=200 (遅い), backup=5。hedge_delay ≈ 95 → 95+5=100 vs 200 → 100
        assert!((hedge_delay - 95.05).abs() < 1.0, "hedge_delay was {hedge_delay}");
        assert!((effective - (hedge_delay + 5.0)).abs() < 1e-6);
    }

    #[test]
    fn adaptive_hedge_takes_primary_if_fast() {
        let samples: Vec<f64> = (1..=100).map(|x| x as f64).collect();
        // primary が hedge_delay より速ければ primary を採用
        let (effective, _) = adaptive_hedge_one_request(20.0, 5.0, &samples);
        assert!((effective - 20.0).abs() < f64::EPSILON);
    }
}
