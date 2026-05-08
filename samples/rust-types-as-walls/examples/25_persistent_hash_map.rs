//! `HashMap` を純粋 update で扱う例。
//! `12_friction_ownership.rs` の「共有しながら更新したい」問題に対して、
//! `im-rc::HashMap` なら古い map を残したまま新しい map を返せる。

#![allow(
    clippy::panic,
    clippy::print_stdout,
    reason = "examples print their narrative output and may use unreachable demo branches"
)]

use std::collections::HashMap as StdHashMap;

use im_rc::HashMap;

fn update_std(
    lines: &StdHashMap<String, u32>,
    sku: &str,
    quantity: u32,
) -> StdHashMap<String, u32> {
    let mut next_lines = lines.clone();
    let current = next_lines.entry(sku.to_owned()).or_insert(0);
    *current += quantity;
    next_lines
}

fn update_persistent(
    lines: &HashMap<String, u32>,
    sku: &str,
    quantity: u32,
) -> HashMap<String, u32> {
    lines.update_with(sku.to_owned(), quantity, |current, added| current + added)
}

fn main() {
    let mut std_before = StdHashMap::new();
    std_before.insert("BOOK-001".to_owned(), 1);
    let std_after = update_std(&std_before, "BOOK-001", 2);

    let persistent_before = HashMap::new()
        .update("BOOK-001".to_owned(), 1)
        .update("PEN-001".to_owned(), 3);
    let persistent_shared = persistent_before.clone();
    assert!(persistent_before.ptr_eq(&persistent_shared));
    let persistent_after = update_persistent(&persistent_shared, "BOOK-001", 2);

    println!("std::collections::HashMap: before={std_before:?} / after={std_after:?}");
    println!("im_rc::HashMap: before={persistent_before:?} / after={persistent_after:?}");
    println!(
        "構造共有 clone の ptr_eq={} / before の BOOK-001 は {:?}",
        persistent_before.ptr_eq(&persistent_shared),
        persistent_before.get("BOOK-001").copied()
    );

    assert_eq!(std_before.get("BOOK-001").copied(), Some(1));
    assert_eq!(std_after.get("BOOK-001").copied(), Some(3));
    assert_eq!(persistent_before.get("BOOK-001").copied(), Some(1));
    assert_eq!(persistent_after.get("BOOK-001").copied(), Some(3));
}
