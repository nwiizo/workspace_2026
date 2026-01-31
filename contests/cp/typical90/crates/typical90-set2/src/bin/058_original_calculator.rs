// 058 - Original Calculator (★4)
// https://atcoder.jp/contests/typical90/tasks/typical90_bf
//
// K が最大 10^18 なのでサイクル検出が必要
// 値の範囲が 0 ~ 99999 なので、最大 10^5 回でサイクルに入る

use proconio::input;

const MOD: u64 = 100_000;

fn main() {
    input! {
        n: u64,
        k: u64,
    }
    println!("{}", solve(n, k));
}

fn solve(n: u64, k: u64) -> u64 {
    // 各値から次の値への遷移を記録
    let mut visited = vec![-1i64; MOD as usize];
    let mut history = Vec::new();

    let mut current = n;
    let mut step = 0u64;

    // サイクル検出
    while visited[current as usize] == -1 {
        visited[current as usize] = step as i64;
        history.push(current);

        if step == k {
            return current;
        }

        current = next_value(current);
        step += 1;
    }

    // サイクル発見
    let cycle_start = visited[current as usize] as u64;
    let cycle_len = step - cycle_start;

    if k < cycle_start {
        return history[k as usize];
    }

    // サイクル内の位置を計算
    let pos_in_cycle = (k - cycle_start) % cycle_len;
    history[(cycle_start + pos_in_cycle) as usize]
}

fn next_value(x: u64) -> u64 {
    let digit_sum = digit_sum(x);
    (x + digit_sum) % MOD
}

fn digit_sum(mut x: u64) -> u64 {
    let mut sum = 0;
    while x > 0 {
        sum += x % 10;
        x /= 10;
    }
    sum
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_example1() {
        // 5 → 5+5=10 → 10+1=11 → 11+2=13
        assert_eq!(solve(5, 3), 13);
    }

    #[test]
    fn test_example2() {
        // 0 → 0+0=0 (ずっと0)
        assert_eq!(solve(0, 100), 0);
    }

    #[test]
    fn test_example3() {
        assert_eq!(solve(99999, 1_000_000_000_000_000_000), 84563);
    }
}
