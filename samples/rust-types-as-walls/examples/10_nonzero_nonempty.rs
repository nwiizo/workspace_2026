//! 既製の制約型を活用する。
//! 標準ライブラリの `NonZero<T>` (Rust 2024 edition 対応のジェネリック形式) など、
//! 制約が型に埋め込まれた型をまず検討する。
//!
//! スライド「既製の制約型を積極的に使う」に対応。

#![allow(
    clippy::panic,
    clippy::print_stdout,
    reason = "examples print their narrative output and may use unreachable demo branches"
)]

use std::num::NonZero;

/// 在庫数は0であってはならない、という制約を型で表現する。
fn divide_stock(total: u32, chunks: NonZero<u32>) -> u32 {
    total.div_euclid(chunks.get())
    // 0除算が発生しないことが、型レベルで保証されている。
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // NonZero<u32> のジェネリック形式（2024 edition で推奨）
    let Some(chunks): Option<NonZero<u32>> = NonZero::new(4) else {
        return Err(std::io::Error::other("4 はゼロではない").into());
    };
    println!(
        "総在庫 100 を {} に分割: {} 個",
        chunks.get(),
        divide_stock(100, chunks)
    );

    // 旧来の型エイリアス NonZeroU32 も引き続き利用可能
    let Some(also_ok): Option<std::num::NonZeroU32> = std::num::NonZeroU32::new(4) else {
        return Err(std::io::Error::other("4 はゼロではない").into());
    };
    println!("別名経由でも: {}", also_ok.get());

    // NonZero::new は Option を返す。ゼロを渡すと None。
    assert!(NonZero::<u32>::new(0).is_none());

    // 次の行のコメントを外すとコンパイルエラー:
    //   error[E0308]: mismatched types
    //   expected `NonZero<u32>`, found `u32`
    // let bad = divide_stock(100, 0);

    // 既製の制約型の例:
    // - `String` / `&str`: 必ず UTF-8 として有効
    // - `Option<&T>`: null不可の参照を安全に表現
    // - `std::num::NonZero<T>`: ゼロではない整数
    // エコシステム:
    // - `nonempty::NonEmpty<T>`: 空でないコレクション
    // - `nutype`: マクロで newtype + バリデーションを一括生成
    // - `validator`: フィールド単位のバリデーション属性

    Ok(())
}
