// 027 - Sign Up Requests (★2)
// https://atcoder.jp/contests/typical90/tasks/typical90_aa
//
// 問題: N個のユーザー名申請がある。各申請について、
//       それ以前に同じ名前がなければ受理。受理された申請番号を出力。
//
// 解法: HashSetで既出チェック

use proconio::input;
use std::collections::HashSet;

fn main() {
    input! {
        n: usize,
        names: [String; n],
    }
    solve(&names);
}

fn solve(names: &[String]) {
    let mut seen = HashSet::new();

    for (i, name) in names.iter().enumerate() {
        if seen.insert(name.clone()) {
            println!("{}", i + 1);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    #[test]
    fn test_logic() {
        let names = vec![
            "alice".to_string(),
            "bob".to_string(),
            "alice".to_string(),
            "charlie".to_string(),
        ];

        let mut seen = HashSet::new();
        let mut accepted = vec![];

        for (i, name) in names.iter().enumerate() {
            if seen.insert(name.clone()) {
                accepted.push(i + 1);
            }
        }

        // alice(1), bob(2), charlie(4) が受理
        assert_eq!(accepted, vec![1, 2, 4]);
    }
}
