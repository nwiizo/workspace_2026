// 027 - Sign Up Requests (★2)
// https://atcoder.jp/contests/typical90/tasks/typical90_aa
//
// ============================================================================
// 【物語で理解する問題】
// ============================================================================
//
// ウェブサービスの新規登録システムを作っています。
//
// ユーザーが登録申請を送ってきます。
// ルール:
// - 同じユーザー名は登録できない（先着順）
// - 最初に来た申請だけが受理される
//
// 各申請について、受理されたものの番号を出力してください。
//
// 例:
//   申請1: "alice"  → 受理（初めて）
//   申請2: "bob"    → 受理（初めて）
//   申請3: "alice"  → 却下（既に登録済み）
//   申請4: "charlie"→ 受理（初めて）
//
// 出力: 1 2 4
//
// ============================================================================
// 【解法：HashSet で既出判定】
// ============================================================================
//
// 【データ構造の選択】
//
// 「既に見たか？」を高速に判定したい。
// → HashSet が最適！（O(1) で判定・追加）
//
// 【アルゴリズム】
//
// 1. 空の HashSet を用意
// 2. 各申請について:
//    - HashSet に名前がなければ → 追加して受理
//    - 既にあれば → 却下
//
// 【HashSet.insert() の便利な性質】
//
// Rust の HashSet::insert() は:
// - 新規追加なら true を返す
// - 既存なら false を返す
//
// この性質を使うと、1行で判定と追加ができる！
//
// ============================================================================
// 【計算量】
// ============================================================================
//
// - 各申請: O(|name|) （ハッシュ計算）
// - 合計: O(N × |name|)
//
// ============================================================================

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
    // -------------------------------------------------------------------------
    // 【HashSet で既出管理】
    //
    // 既に受理した名前を記録
    // -------------------------------------------------------------------------
    let mut seen = HashSet::new();

    for (i, name) in names.iter().enumerate() {
        // ---------------------------------------------------------------------
        // 【insert() で判定と追加を同時に】
        //
        // - 新規追加（受理）なら true
        // - 既存（却下）なら false
        // ---------------------------------------------------------------------
        if seen.insert(name.clone()) {
            // 受理: 1-indexed で出力
            println!("{}", i + 1);
        }
    }
}

// =============================================================================
// 【テスト】
// =============================================================================
#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    #[test]
    fn basic_test() {
        let names = vec![
            "alice".to_string(),
            "bob".to_string(),
            "alice".to_string(), // 重複
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
        // alice(3) は却下
        assert_eq!(accepted, vec![1, 2, 4]);
    }

    #[test]
    fn all_unique() {
        // 全て異なる名前 → 全て受理
        let names: Vec<String> = vec!["a", "b", "c", "d"]
            .into_iter()
            .map(|s| s.to_string())
            .collect();

        let mut seen = HashSet::new();
        let mut accepted = vec![];

        for (i, name) in names.iter().enumerate() {
            if seen.insert(name.clone()) {
                accepted.push(i + 1);
            }
        }

        assert_eq!(accepted, vec![1, 2, 3, 4]);
    }

    #[test]
    fn all_same() {
        // 全て同じ名前 → 最初の1つだけ受理
        let names: Vec<String> = vec!["same", "same", "same", "same"]
            .into_iter()
            .map(|s| s.to_string())
            .collect();

        let mut seen = HashSet::new();
        let mut accepted = vec![];

        for (i, name) in names.iter().enumerate() {
            if seen.insert(name.clone()) {
                accepted.push(i + 1);
            }
        }

        assert_eq!(accepted, vec![1]);
    }

    #[test]
    fn alternating() {
        // 交互に同じ名前
        let names: Vec<String> = vec!["a", "b", "a", "b", "c"]
            .into_iter()
            .map(|s| s.to_string())
            .collect();

        let mut seen = HashSet::new();
        let mut accepted = vec![];

        for (i, name) in names.iter().enumerate() {
            if seen.insert(name.clone()) {
                accepted.push(i + 1);
            }
        }

        // a(1), b(2), c(5) が受理
        assert_eq!(accepted, vec![1, 2, 5]);
    }

    #[test]
    fn single_request() {
        // 1つだけ → 必ず受理
        let names = vec!["onlyone".to_string()];

        let mut seen = HashSet::new();
        let mut accepted = vec![];

        for (i, name) in names.iter().enumerate() {
            if seen.insert(name.clone()) {
                accepted.push(i + 1);
            }
        }

        assert_eq!(accepted, vec![1]);
    }
}
