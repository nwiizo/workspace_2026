// 053 - Discrete Dowsing (★7)
// https://atcoder.jp/contests/typical90/tasks/typical90_ba
//
// ============================================================
// インタラクティブ問題 - 三分探索
// ============================================================
//
// 山型配列（最初は単調増加、途中から単調減少）の最大値を見つける
//
// 三分探索:
// - 区間を3分割する2点 m1, m2 を選ぶ
// - A[m1] < A[m2] なら最大値は右側にある
// - A[m1] > A[m2] なら最大値は左側にある
//
// クエリ数: O(log N) ≈ 25回程度 (N=1500)
//
// ============================================================

use std::io::{self, BufRead, Write};

fn main() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut reader = stdin.lock();
    let mut writer = stdout.lock();

    // テストケース数を読み取り
    let t: usize = read_line(&mut reader).trim().parse().unwrap();

    for _ in 0..t {
        // 配列長を読み取り
        let n: usize = read_line(&mut reader).trim().parse().unwrap();

        // 三分探索
        let max_val = ternary_search(n, &mut reader, &mut writer);

        // 答えを出力
        writeln!(writer, "! {}", max_val).unwrap();
        writer.flush().unwrap();
    }
}

fn ternary_search<R: BufRead, W: Write>(n: usize, reader: &mut R, writer: &mut W) -> i64 {
    let mut lo = 1usize;
    let mut hi = n;

    // クエリ結果をキャッシュ
    let mut cache = std::collections::HashMap::new();

    let query = |pos: usize,
                 cache: &mut std::collections::HashMap<usize, i64>,
                 reader: &mut R,
                 writer: &mut W|
     -> i64 {
        if let Some(&val) = cache.get(&pos) {
            return val;
        }
        writeln!(writer, "? {}", pos).unwrap();
        writer.flush().unwrap();
        let val: i64 = read_line(reader).trim().parse().unwrap();
        cache.insert(pos, val);
        val
    };

    while hi - lo >= 2 {
        let m1 = lo + (hi - lo) / 3;
        let m2 = hi - (hi - lo) / 3;

        let v1 = query(m1, &mut cache, reader, writer);
        let v2 = query(m2, &mut cache, reader, writer);

        if v1 < v2 {
            lo = m1 + 1;
        } else {
            hi = m2 - 1;
        }
    }

    // 残りの区間で最大値を見つける
    let mut max_val = 0i64;
    for pos in lo..=hi {
        let val = query(pos, &mut cache, reader, writer);
        max_val = max_val.max(val);
    }

    max_val
}

fn read_line<R: BufRead>(reader: &mut R) -> String {
    let mut line = String::new();
    reader.read_line(&mut line).unwrap();
    line
}

// インタラクティブ問題のためテストは省略
#[cfg(test)]
mod tests {
    #[test]
    fn test_logic() {
        // 三分探索のロジックテスト（オフライン）
        let arr = vec![1, 3, 5, 7, 9, 8, 6, 4, 2];
        let n = arr.len();
        let mut lo = 0;
        let mut hi = n - 1;

        while hi - lo >= 2 {
            let m1 = lo + (hi - lo) / 3;
            let m2 = hi - (hi - lo) / 3;

            if arr[m1] < arr[m2] {
                lo = m1 + 1;
            } else {
                hi = m2 - 1;
            }
        }

        let max_val = (lo..=hi).map(|i| arr[i]).max().unwrap();
        assert_eq!(max_val, 9);
    }
}
