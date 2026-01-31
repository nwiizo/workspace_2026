// 047 - Monochromatic Diagonal (★7)
// https://atcoder.jp/contests/typical90/tasks/typical90_au
//
// ============================================================
// 問題の説明（わかりやすく）
// ============================================================
//
// 【設定】
// - 文字列 S と T がそれぞれ N 文字ある（文字は R, G, B のみ）
// - N×N のマス目を作る
// - マス (i, j) の色は以下のルールで決まる:
//   - S[i] と T[j] が同じ文字 → その文字の色
//   - S[i] と T[j] が違う文字 → 残りの1色（第三色）
//
// 【例】
// S[0] = 'R', T[1] = 'G' のとき
// → R と G は違う → 残りは B → マス(0,1)は青色
//
// 【対角線とは】
// 左上から右下に向かう斜めの線。マス目では:
// - 対角線0: (0,0), (1,1), (2,2), ...
// - 対角線1: (0,1), (1,2), (2,3), ...
// - 対角線-1: (1,0), (2,1), (3,2), ...
//
// 【目標】
// すべてのマスが同じ色の対角線を「単色対角線」と呼ぶ
// 単色対角線の本数を数える
//
// ============================================================
// 解法のアイデア（なぜこう解くのか）
// ============================================================
//
// 【素朴な方法】
// 各対角線について、すべてのマスの色を計算して比較
// → O(N²) で N=10^6 だと間に合わない
//
// 【高速化の鍵】
// 色の計算に「魔法の変換」を使う！
//
// R=0, G=1, B=2 と数字で表すと、色の計算は:
// - S[i] == T[j] なら色 = S[i]
// - S[i] != T[j] なら色 = 3 - S[i] - T[j]
//
// これを「2倍して3で割った余り」で変換すると:
// f(0)=0, f(1)=2, f(2)=1
// すると色 = (f(S[i]) + f(T[j])) % 3 と書ける！
//
// 【なぜこれが嬉しいか】
// 対角線上で色が全部同じ
// = f(S[0])+f(T[k]) = f(S[1])+f(T[k+1]) = ... (mod 3)
// = 隣同士の差がゼロ
// = (S[1]-S[0]) + (T[k+1]-T[k]) = 0 (mod 3)
//
// つまり「S の差分列」と「T の差分列の符号反転」が一致すれば良い！
// これは Z-algorithm で O(N) で判定できる
//
// ============================================================
// Z-algorithm とは
// ============================================================
//
// 文字列 S に対して、S[i..] と S[0..] が何文字一致するかを
// すべての位置 i について O(N) で計算するアルゴリズム
//
// 例: S = "aabaa"
// z[0] = 5 (自分自身なので全部一致)
// z[1] = 1 (S[1..] = "abaa" と S = "aabaa" の共通接頭辞は "a")
// z[2] = 0 (S[2..] = "baa" と S = "aabaa" は最初から不一致)
// z[3] = 2 (S[3..] = "aa" と S = "aabaa" の共通接頭辞は "aa")
// z[4] = 1 (S[4..] = "a" と S = "aabaa" の共通接頭辞は "a")
//
// ============================================================

use proconio::input;

fn main() {
    input! {
        n: usize,
        s: String,
        t: String,
    }
    println!("{}", solve(n, &s, &t));
}

fn solve(n: usize, s: &str, t: &str) -> usize {
    // ステップ1: 文字を数字に変換
    // R → 0, G → 1, B → 2
    let char_to_num = |c: char| -> usize {
        match c {
            'R' => 0,
            'G' => 1,
            'B' => 2,
            _ => panic!("想定外の文字です"),
        }
    };

    let s: Vec<usize> = s.chars().map(char_to_num).collect();
    let t: Vec<usize> = t.chars().map(char_to_num).collect();

    // ステップ2: 魔法の変換を適用
    // f(x) = (2 * x) % 3
    // f(0) = 0, f(1) = 2, f(2) = 1
    //
    // この変換により、マスの色が (f(S[i]) + f(T[j])) % 3 で計算できる
    let transform = |x: usize| -> usize { (2 * x) % 3 };
    let s_transformed: Vec<usize> = s.iter().map(|&x| transform(x)).collect();
    let t_transformed: Vec<usize> = t.iter().map(|&x| transform(x)).collect();

    // 特殊ケース: N=1 のとき対角線は1本だけで、マスも1個だけなので必ず単色
    if n == 1 {
        return 1;
    }

    // ステップ3: 差分列を作る
    //
    // 対角線が単色になる条件:
    // (S'[i+1] - S'[i]) + (T'[j+1] - T'[j]) ≡ 0 (mod 3)
    //
    // これを書き換えると:
    // (S'[i+1] - S'[i]) ≡ -(T'[j+1] - T'[j]) ≡ (T'[j] - T'[j+1]) (mod 3)
    //
    // つまり:
    // - ds[i] = S'[i+1] - S'[i] (S の差分)
    // - et[j] = T'[j] - T'[j+1] (T の逆差分)
    // この2つが一致すれば対角線は単色！

    // S の差分列を計算
    // ds[i] = (S'[i+1] - S'[i] + 3) % 3
    // (負の数を避けるため +3 してから %3)
    let ds: Vec<usize> = (0..n - 1)
        .map(|i| (s_transformed[i + 1] + 3 - s_transformed[i]) % 3)
        .collect();

    // T の逆差分列を計算
    // et[j] = (T'[j] - T'[j+1] + 3) % 3
    let et: Vec<usize> = (0..n - 1)
        .map(|j| (t_transformed[j] + 3 - t_transformed[j + 1]) % 3)
        .collect();

    // ステップ4: Z-algorithm で文字列マッチングを行う
    //
    // 対角線 k が単色 ⟺ ds と et (をk個ずらしたもの) が一致
    //
    // これを効率的に調べるため:
    // - z1: "ds + # + et" に対する Z-algorithm
    //   → et の各位置から ds と何文字一致するかがわかる
    // - z2: "et + # + ds" に対する Z-algorithm
    //   → ds の各位置から et と何文字一致するかがわかる

    // z1 の構築: ds + separator + et
    let mut z_str1: Vec<usize> = Vec::with_capacity(2 * n);
    z_str1.extend(&ds);
    z_str1.push(3); // 区切り文字 (0,1,2 以外の値)
    z_str1.extend(&et);
    let z1 = z_algorithm(&z_str1);

    // z2 の構築: et + separator + ds
    let mut z_str2: Vec<usize> = Vec::with_capacity(2 * n);
    z_str2.extend(&et);
    z_str2.push(3);
    z_str2.extend(&ds);
    let z2 = z_algorithm(&z_str2);

    // ステップ5: 各対角線について判定
    let mut count = 0;

    // 対角線は k = -(N-1) から k = N-1 まである
    for k in (-(n as i64) + 1)..=(n as i64 - 1) {
        // この対角線の長さ（マスの数）
        let diagonal_length = n - k.unsigned_abs() as usize;

        // 長さ1の対角線は必ず単色（マスが1個しかないので）
        if diagonal_length == 1 {
            count += 1;
            continue;
        }

        // 差分列の一致長を調べる
        // 必要な一致長は (diagonal_length - 1) 文字
        let required_match = diagonal_length - 1;
        let actual_match: usize;

        if k >= 0 {
            // k >= 0 の対角線: ds[0..] と et[k..] を比較
            let ku = k as usize;
            if ku >= n - 1 {
                // et[k..] が空なので一致長は0
                actual_match = 0;
            } else {
                // z1 の構造: [ds..., #, et...]
                // z1[|ds| + 1 + k] = et[k..] と ds の一致長
                let idx = ds.len() + 1 + ku;
                actual_match = if idx < z1.len() { z1[idx] } else { 0 };
            }
        } else {
            // k < 0 の対角線: ds[|k|..] と et[0..] を比較
            let m = (-k) as usize;
            if m >= n - 1 {
                actual_match = 0;
            } else {
                // z2 の構造: [et..., #, ds...]
                // z2[|et| + 1 + m] = ds[m..] と et の一致長
                let idx = et.len() + 1 + m;
                actual_match = if idx < z2.len() { z2[idx] } else { 0 };
            }
        }

        // 一致長が必要な長さ以上なら単色
        if actual_match >= required_match {
            count += 1;
        }
    }

    count
}

/// Z-algorithm の実装
///
/// 入力: 配列 s (長さ n)
/// 出力: 配列 z (長さ n)
///   z[i] = s[0..] と s[i..] の最長共通接頭辞の長さ
///
/// 計算量: O(n)
fn z_algorithm(s: &[usize]) -> Vec<usize> {
    let n = s.len();
    if n == 0 {
        return vec![];
    }

    let mut z = vec![0; n];
    z[0] = n; // s[0..] と s[0..] は完全一致

    // l, r は「今まで見つけた中で最も右に伸びている一致区間」を表す
    let mut l = 0;
    let mut r = 0;

    for i in 1..n {
        // i < r なら、以前の計算結果を再利用できる
        if i < r {
            // z[i - l] は s[i-l..] と s[0..] の一致長
            // これは s[i..] と s[0..] の一致長の参考になる
            z[i] = std::cmp::min(r - i, z[i - l]);
        }

        // 実際に文字を比較して一致長を延ばす
        while i + z[i] < n && s[z[i]] == s[i + z[i]] {
            z[i] += 1;
        }

        // 一致区間がより右に伸びたら更新
        if i + z[i] > r {
            l = i;
            r = i + z[i];
        }
    }

    z
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_example1() {
        // S = "RGBGB", T = "GRGRB"
        // 単色対角線は k = -4, -3, -1, 2, 3, 4 の6本
        assert_eq!(solve(5, "RGBGB", "GRGRB"), 6);
    }

    #[test]
    fn test_example2() {
        // S = "RRR", T = "BBB"
        // すべての対角線が単色
        assert_eq!(solve(3, "RRR", "BBB"), 5);
    }

    #[test]
    fn test_example3() {
        assert_eq!(solve(10, "BGGGRBBGRG", "RGBBRGRGGG"), 4);
    }
}
