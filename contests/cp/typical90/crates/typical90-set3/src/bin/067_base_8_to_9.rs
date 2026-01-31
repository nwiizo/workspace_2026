// 067 - Base 8 to 9 (★2)
// https://atcoder.jp/contests/typical90/tasks/typical90_bo
//
// 8進数 → 9進数に変換 → 8を5に置換 → 8進数として扱う
// これをK回繰り返す

use proconio::input;

fn main() {
    input! {
        n: String,
        k: usize,
    }
    println!("{}", solve(&n, k));
}

fn solve(n: &str, k: usize) -> String {
    // 8進数文字列を数値のベクタに変換
    let mut digits: Vec<u64> = n.chars().map(|c| c.to_digit(10).unwrap() as u64).collect();

    for _ in 0..k {
        // 8進数を10進数に変換
        let mut value = 0u64;
        for &d in &digits {
            value = value * 8 + d;
        }

        if value == 0 {
            digits = vec![0];
            continue;
        }

        // 10進数を9進数に変換
        let mut base9_digits = Vec::new();
        while value > 0 {
            let d = value % 9;
            // 8を5に置換
            base9_digits.push(if d == 8 { 5 } else { d });
            value /= 9;
        }
        base9_digits.reverse();
        digits = base9_digits;
    }

    // 結果を文字列に変換
    if digits.is_empty() {
        "0".to_string()
    } else {
        digits.iter().map(|&d| (d as u8 + b'0') as char).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_example1() {
        assert_eq!(solve("21", 1), "15");
    }

    #[test]
    fn test_example2() {
        assert_eq!(solve("1330", 1), "555");
    }

    #[test]
    fn test_example3() {
        assert_eq!(solve("2311640221315", 15), "474547");
    }
}
