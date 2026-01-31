// 061 - Deck (★2)
// https://atcoder.jp/contests/typical90/tasks/typical90_bi
//
// VecDeque で両端への挿入とランダムアクセス

use proconio::input;
use std::collections::VecDeque;

fn main() {
    input! {
        q: usize,
    }

    let mut deck: VecDeque<u64> = VecDeque::new();

    for _ in 0..q {
        input! {
            t: usize,
            x: u64,
        }

        match t {
            1 => {
                // 最上部に挿入
                deck.push_front(x);
            }
            2 => {
                // 最下部に挿入
                deck.push_back(x);
            }
            3 => {
                // 上からx番目を出力 (1-indexed)
                println!("{}", deck[(x - 1) as usize]);
            }
            _ => unreachable!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    #[test]
    fn test_example1() {
        let mut deck: VecDeque<u64> = VecDeque::new();

        // 操作1: t=1, x=1 → [1]
        deck.push_front(1);

        // 操作2: t=1, x=2 → [2, 1]
        deck.push_front(2);

        // 操作3: t=2, x=3 → [2, 1, 3]
        deck.push_back(3);

        // 操作4: t=3, x=2 → 出力: 1
        assert_eq!(deck[1], 1);

        // 操作5: t=1, x=1 → [1, 2, 1, 3]
        deck.push_front(1);

        // 操作6: t=3, x=1 → 出力: 1
        assert_eq!(deck[0], 1);
    }
}
