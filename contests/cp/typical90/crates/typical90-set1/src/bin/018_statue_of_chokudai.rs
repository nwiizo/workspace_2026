// 018 - Statue of Chokudai (★3)
// https://atcoder.jp/contests/typical90/tasks/typical90_r
//
// 問題: 半径Rの円周上を角速度ωで回転する点がある。
//       観測点から見た仰角を各クエリ時刻について求めよ。
//
// 解法: 三角関数
//       時刻tでの角度 = -π/2 + 2π*t/T (T=1周の時間)
//       y座標 = R * sin(角度)
//       仰角 = atan2(y - C, X) を度数法で出力

use proconio::input;
use std::f64::consts::PI;

fn main() {
    input! {
        t: f64,     // 1周にかかる時間
        l: f64,     // 観測点から回転軸までの距離
        x: f64,     // 回転の中心の高さ
        q: usize,
    }

    for _ in 0..q {
        input! { e: f64 }
        let angle = calc_elevation(t, l, x, e);
        println!("{}", angle);
    }
}

fn calc_elevation(period: f64, dist: f64, center_height: f64, time: f64) -> f64 {
    // 時刻tでの回転角（ラジアン）
    // t=0で最下点（-π/2）から始まり、時計回りに回転
    let theta = 2.0 * PI * time / period - PI / 2.0;

    // y座標（高さ）= 中心の高さ + 半径 * sin(θ)
    // ただし半径=1（問題の制約から中心からの距離）
    // 実際には X が中心の高さで、回転半径は別途与えられる
    // 問題文を再確認: 半径1の円周上を回転、中心の高さがX

    let y = center_height + theta.sin(); // 半径は1

    // 仰角 = atan2(高さ, 水平距離)
    let elevation_rad = (y / dist).atan();
    elevation_rad * 180.0 / PI
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-6
    }

    #[test]
    fn test_at_bottom() {
        // t=0 で最下点: y = X - 1
        // 仰角 = atan((X-1)/L)
        let angle = calc_elevation(4.0, 10.0, 5.0, 0.0);
        let expected = ((5.0 - 1.0) / 10.0_f64).atan() * 180.0 / PI;
        assert!(approx_eq(angle, expected));
    }

    #[test]
    fn test_at_top() {
        // t=T/2 で最上点: y = X + 1
        let angle = calc_elevation(4.0, 10.0, 5.0, 2.0);
        let expected = ((5.0 + 1.0) / 10.0_f64).atan() * 180.0 / PI;
        assert!(approx_eq(angle, expected));
    }
}
