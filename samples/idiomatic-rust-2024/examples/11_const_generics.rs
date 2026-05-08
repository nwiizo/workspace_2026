//! ch7 / 7.1 Const generics。
//!
//! 2024 edition で適用したベストプラクティス:
//! - 配列サイズや次元を const generic で型に乗せる
//! - 1.83+ で const fn 可能領域が広がり、コンパイル時計算で安全に組める
//! - `NonZero<T>` (2024) で「ゼロでない」を型で示す idiom もここで触れる

use std::num::NonZero;

#[derive(Debug)]
pub struct Vector<const N: usize> {
    pub data: [f64; N],
}

impl<const N: usize> Vector<N> {
    pub const fn zero() -> Self {
        Self { data: [0.0; N] }
    }

    pub fn dot(&self, other: &Self) -> f64 {
        let mut acc = 0.0;
        for i in 0..N {
            acc += self.data[i] * other.data[i];
        }
        acc
    }
}

// 異なる N 同士は型で混ぜられない: dot は &Self なので N 一致が強制される。

/// Page size は 0 ではありえない。`NonZero<usize>` で型に乗せる。
const fn page_count(total_items: usize, page_size: NonZero<usize>) -> usize {
    total_items.div_ceil(page_size.get())
}

fn main() {
    let a: Vector<3> = Vector {
        data: [1.0, 2.0, 3.0],
    };
    let b: Vector<3> = Vector {
        data: [4.0, 5.0, 6.0],
    };
    println!("dot = {}", a.dot(&b));

    let zero3: Vector<3> = Vector::zero();
    println!("zero = {:?}", zero3.data);

    // ゼロ割り防止: NonZero<usize> はゼロを構築不能。
    // const block で「コンパイル時に panic しないリテラル」を表明する。
    let page_size = const { NonZero::new(50).expect("50 != 0") };
    let pages = page_count(1003, page_size);
    println!("pages = {pages}");
}
