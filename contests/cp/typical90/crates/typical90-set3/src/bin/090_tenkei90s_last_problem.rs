// 090 - Tenkei90's Last Problem (★7)
// https://atcoder.jp/contests/typical90/tasks/typical90_cl
//
// 問題: 長さ N の数列 A (各要素 0 ~ K) で、
// 全ての連続区間 [l, r] について min(A[l..=r]) * (r - l + 1) <= K を満たすものを数える
//
// 解法:
// 制約を言い換える: 各値 v について、v 以上の連続区間の長さは K/v 以下
//
// 状態: 各しきい値 v について、現在の「v 以上の連続長」を追跡
// ただし状態数が爆発するため、到達可能な状態のみを列挙して行列累乗

use proconio::input;
use std::collections::HashMap;

const MOD: u64 = 998244353;

fn main() {
    input! {
        n: u64,
        k: usize,
    }
    println!("{}", solve(n, k));
}

/// 状態を表す型: 各しきい値 v での連続長 (l_1 >= l_2 >= ... >= l_K)
/// 効率化のため、(v, l_v) のペアで l_v が変わる点のみ保持
type State = Vec<(usize, usize)>; // [(v1, l1), (v2, l2), ...] where v1 < v2 < ...

fn state_to_vec(state: &State, k: usize) -> Vec<usize> {
    let mut result = vec![0; k + 1];
    let mut idx = 0;
    let mut current_l = 0;
    for v in 1..=k {
        if idx < state.len() && state[idx].0 == v {
            current_l = state[idx].1;
            idx += 1;
        }
        result[v] = current_l;
    }
    result
}

fn compress_state(l: &[usize]) -> State {
    let mut state = Vec::new();
    let mut prev = 0;
    for (v, &lv) in l.iter().enumerate().skip(1) {
        if lv != prev {
            state.push((v, lv));
            prev = lv;
        }
    }
    state
}

fn solve(n: u64, k: usize) -> u64 {
    if k == 0 {
        return 1;
    }

    // 到達可能な状態を BFS で列挙
    let empty_state: State = vec![];
    let mut state_to_id: HashMap<State, usize> = HashMap::new();
    let mut states: Vec<State> = Vec::new();

    state_to_id.insert(empty_state.clone(), 0);
    states.push(empty_state);

    let mut queue = vec![0usize];
    let mut head = 0;

    while head < queue.len() {
        let sid = queue[head];
        head += 1;

        let state = &states[sid];
        let l = state_to_vec(state, k);

        // 遷移: 0 を置く → 空状態
        // (既に登録済み)

        // 遷移: 非0 の w を置く
        for w in 1..=k {
            // 新しい l' を計算
            let mut new_l = vec![0; k + 1];
            let mut valid = true;
            for v in 1..=w {
                new_l[v] = l[v] + 1;
                if new_l[v] > k / v {
                    valid = false;
                    break;
                }
            }
            if !valid {
                continue;
            }
            // v > w については new_l[v] = 0

            let new_state = compress_state(&new_l);
            if !state_to_id.contains_key(&new_state) {
                let new_id = states.len();
                state_to_id.insert(new_state.clone(), new_id);
                states.push(new_state);
                queue.push(new_id);
            }
        }
    }

    let num_states = states.len();

    // 遷移行列を構築
    let mut mat = vec![vec![0u64; num_states]; num_states];

    for (sid, state) in states.iter().enumerate() {
        let l = state_to_vec(state, k);

        // 0 を置く → 空状態 (id = 0)
        mat[0][sid] = 1;

        // 非0 の w を置く
        for w in 1..=k {
            let mut new_l = vec![0; k + 1];
            let mut valid = true;
            for v in 1..=w {
                new_l[v] = l[v] + 1;
                if new_l[v] > k / v {
                    valid = false;
                    break;
                }
            }
            if !valid {
                continue;
            }

            let new_state = compress_state(&new_l);
            let to_id = state_to_id[&new_state];
            mat[to_id][sid] = (mat[to_id][sid] + 1) % MOD;
        }
    }

    // 初期状態: 空状態から開始
    let mut initial = vec![0u64; num_states];
    initial[0] = 1;

    // N 回遷移
    let result_mat = mat_pow(&mat, n);
    let result = mat_vec_mul(&result_mat, &initial);

    // 全状態の和
    result.iter().fold(0, |acc, &x| (acc + x) % MOD)
}

fn mat_mul(a: &[Vec<u64>], b: &[Vec<u64>]) -> Vec<Vec<u64>> {
    let n = a.len();
    let mut c = vec![vec![0u64; n]; n];
    for i in 0..n {
        for k in 0..n {
            if a[i][k] == 0 {
                continue;
            }
            for j in 0..n {
                c[i][j] = (c[i][j] + a[i][k] * b[k][j]) % MOD;
            }
        }
    }
    c
}

fn mat_pow(mat: &[Vec<u64>], mut exp: u64) -> Vec<Vec<u64>> {
    let n = mat.len();
    let mut result = vec![vec![0u64; n]; n];
    for i in 0..n {
        result[i][i] = 1;
    }
    let mut base = mat.to_vec();

    while exp > 0 {
        if exp & 1 == 1 {
            result = mat_mul(&result, &base);
        }
        base = mat_mul(&base, &base);
        exp >>= 1;
    }
    result
}

fn mat_vec_mul(mat: &[Vec<u64>], vec: &[u64]) -> Vec<u64> {
    let n = mat.len();
    let mut result = vec![0u64; n];
    for i in 0..n {
        for j in 0..n {
            result[i] = (result[i] + mat[i][j] * vec[j]) % MOD;
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_example1() {
        assert_eq!(solve(2, 2), 8);
    }

    #[test]
    fn test_example2() {
        assert_eq!(solve(17, 29), 263173793);
    }
}
