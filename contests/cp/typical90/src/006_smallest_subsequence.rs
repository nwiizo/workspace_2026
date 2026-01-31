// 006 - Smallest Subsequence (★5)
// https://atcoder.jp/contests/typical90/tasks/typical90_f
//
// 問題: 文字列SからK文字を選んで作れる辞書順最小の部分列を求めよ。
//
// 解法: 貪欲法
// - 各位置で「この文字を選んでも残りでK文字作れるか」を確認
// - 作れるなら辞書順最小の文字を選ぶ
// - 単調スタックを使う方法もある

use proconio::input;
use proconio::marker::Chars;

fn main() {
    input! {
        n: usize,
        k: usize,
        s: Chars,
    }
    println!("{}", solve(n, k, &s));
}

#[allow(clippy::needless_range_loop)]
fn solve(n: usize, k: usize, s: &[char]) -> String {
    // next[i][c] = 位置i以降で文字cが最初に現れる位置
    // 前計算で O(N * 26)

    // 各文字の次の出現位置を計算
    let mut next = vec![vec![n; 26]; n + 1];
    for i in (0..n).rev() {
        // i+1 の情報をコピー
        for c in 0..26 {
            next[i][c] = next[i + 1][c];
        }
        // 現在の文字を更新
        let c = (s[i] as u8 - b'a') as usize;
        next[i][c] = i;
    }

    let mut result = Vec::with_capacity(k);
    let mut pos = 0; // 現在の探索開始位置

    for remaining in (1..=k).rev() {
        // 残り remaining 文字を選ぶ必要がある
        // 位置 pos 以降から1文字選ぶ

        // この文字を選んだ後、残り remaining-1 文字を選べる必要がある
        // つまり、選ぶ位置は n - (remaining - 1) = n - remaining + 1 以下

        let limit = n - remaining + 1;

        // pos から limit までで辞書順最小の文字を選ぶ
        for c in 0..26 {
            let char_pos = next[pos][c];
            if char_pos < limit {
                result.push((b'a' + c as u8) as char);
                pos = char_pos + 1;
                break;
            }
        }
    }

    result.iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_example1() {
        // "abcdef" から 3文字 → "abc"
        let s: Vec<char> = "abcdef".chars().collect();
        assert_eq!(solve(6, 3, &s), "abc");
    }

    #[test]
    fn test_example2() {
        // "bacba" から 3文字 → "aba"
        let s: Vec<char> = "bacba".chars().collect();
        assert_eq!(solve(5, 3, &s), "aba");
    }

    #[test]
    fn test_example3() {
        // "cba" から 2文字 → "ba"
        let s: Vec<char> = "cba".chars().collect();
        assert_eq!(solve(3, 2, &s), "ba");
    }
}
