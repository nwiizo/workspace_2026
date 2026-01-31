// 084 - There are two types of characters (★3)
// https://atcoder.jp/contests/typical90/tasks/typical90_cf
//
// 両方の文字を含む部分文字列の数を数える
// = 全ての部分文字列 - 'o' のみの部分文字列 - 'x' のみの部分文字列
//
// 全ての部分文字列: N*(N+1)/2
// 同じ文字が連続する区間の長さを L とすると、その区間内の部分文字列は L*(L+1)/2

use proconio::input;

fn main() {
    input! {
        n: usize,
        s: String,
    }
    println!("{}", solve(n, &s));
}

fn solve(n: usize, s: &str) -> u64 {
    let chars: Vec<char> = s.chars().collect();

    // 全ての部分文字列の数
    let total = n as u64 * (n as u64 + 1) / 2;

    // 同じ文字だけの部分文字列の数
    let mut same_only = 0u64;

    let mut i = 0;
    while i < n {
        let c = chars[i];
        let mut j = i;
        while j < n && chars[j] == c {
            j += 1;
        }
        let len = (j - i) as u64;
        same_only += len * (len + 1) / 2;
        i = j;
    }

    total - same_only
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_example1() {
        assert_eq!(solve(4, "ooxo"), 5);
    }

    #[test]
    fn test_example2() {
        assert_eq!(solve(5, "oxoxo"), 10);
    }

    #[test]
    fn test_example3() {
        assert_eq!(solve(5, "ooooo"), 0);
    }

    #[test]
    fn test_example4() {
        assert_eq!(solve(7, "xxoooxx"), 16);
    }
}
