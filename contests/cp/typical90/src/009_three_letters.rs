// 009 - Three Letters (★2)
// https://atcoder.jp/contests/typical90/tasks/typical90_i
//
// 問題: N個の文字列から3つの異なる文字列を選び、
// 各文字列の先頭3文字を連結した9文字の文字列を作る。
// 辞書順最大のものを求めよ。
//
// 解法: ソートして上位3つを選ぶ
// - 各文字列の先頭3文字を取得
// - 降順ソートして上位3つを連結

use proconio::input;

fn main() {
    input! {
        n: usize,
        s: [String; n],
    }
    println!("{}", solve(n, &s));
}

fn solve(n: usize, s: &[String]) -> String {
    // 先頭3文字を取得（3文字未満の場合はそのまま）
    let mut prefixes: Vec<String> = s.iter().map(|t| t.chars().take(3).collect()).collect();

    // 降順ソート
    prefixes.sort_by(|a, b| b.cmp(a));

    // 上位3つを連結
    if n >= 3 {
        format!("{}{}{}", prefixes[0], prefixes[1], prefixes[2])
    } else {
        prefixes.join("")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_example1() {
        let s = vec!["abcd".to_string(), "bcde".to_string(), "cdef".to_string()];
        // 降順: cde, bcd, abc → "cdebcdabc"
        assert_eq!(solve(3, &s), "cdebcdabc");
    }

    #[test]
    fn test_example2() {
        let s = vec![
            "zzz".to_string(),
            "yyy".to_string(),
            "xxx".to_string(),
            "www".to_string(),
        ];
        // 降順: zzz, yyy, xxx → "zzzyyyxxx"
        assert_eq!(solve(4, &s), "zzzyyyxxx");
    }
}
