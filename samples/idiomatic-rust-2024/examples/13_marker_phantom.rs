//! ch7 / 7.5 Marker traits + 7.6 Struct tagging。
//!
//! 2024 edition で適用したベストプラクティス:
//! - `PhantomData<T>` で「単位 / 通貨」をコンパイル時にだけ区別する
//! - 実行時オーバーヘッドゼロ。ランタイム値はただの数値
//! - 異なるタグ同士は型で混ざらない

use std::marker::PhantomData;
use std::ops::Add;

#[derive(Debug, Clone, Copy)]
pub struct Currency<C> {
    cents: i64,
    _tag: PhantomData<C>,
}

impl<C> Currency<C> {
    pub const fn new(cents: i64) -> Self {
        Self {
            cents,
            _tag: PhantomData,
        }
    }
    pub const fn cents(self) -> i64 {
        self.cents
    }
}

impl<C> Add for Currency<C> {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self::new(self.cents + rhs.cents)
    }
}

#[derive(Debug)]
pub struct Jpy;
#[derive(Debug)]
pub struct Usd;

fn main() {
    let a: Currency<Jpy> = Currency::new(1_000);
    let b: Currency<Jpy> = Currency::new(500);
    let total = a + b;
    println!("JPY total = {}", total.cents());

    let _u: Currency<Usd> = Currency::new(100);
    // a + _u; // compile error — 通貨が違うものは型で足せない
}
