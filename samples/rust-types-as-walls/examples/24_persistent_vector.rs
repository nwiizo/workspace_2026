//! `Vec` と永続 `Vector` の対比。
//! ネットワーク制限下で取得済みだった `im-rs` 系の `im-rc::Vector` を使い、
//! clone が cheap で更新時に構造共有が効く感触を確認する。

#![allow(
    clippy::panic,
    clippy::print_stdout,
    reason = "examples print their narrative output and may use unreachable demo branches"
)]

use im_rc::Vector;

fn push_with_vec(lines: &[String], new_sku: &str) -> Vec<String> {
    let mut next_lines = lines.to_owned();
    next_lines.push(new_sku.to_owned());
    next_lines
}

fn push_with_vector(lines: &Vector<String>, new_sku: &str) -> Vector<String> {
    let mut next_lines = lines.clone();
    next_lines.push_back(new_sku.to_owned());
    next_lines
}

fn describe_vector(lines: &Vector<String>) -> String {
    lines.iter().cloned().collect::<Vec<_>>().join(", ")
}

fn main() {
    let vec_original = vec!["BOOK-001".to_owned(), "PEN-001".to_owned()];
    let vec_appended = push_with_vec(&vec_original, "BAG-001");

    let mut vector_original = Vector::new();
    vector_original.push_back("BOOK-001".to_owned());
    vector_original.push_back("PEN-001".to_owned());

    let vector_shared = vector_original.clone();
    assert!(vector_original.ptr_eq(&vector_shared));

    let vector_appended = push_with_vector(&vector_shared, "BAG-001");

    println!(
        "Vec: base=[{}] / next=[{}]",
        vec_original.join(", "),
        vec_appended.join(", ")
    );
    println!(
        "Vector: base=[{}] / next=[{}]",
        describe_vector(&vector_original),
        describe_vector(&vector_appended)
    );
    println!(
        "clone 直後は ptr_eq={} / push_back 後も元の長さは {} のまま",
        vector_original.ptr_eq(&vector_shared),
        vector_original.len()
    );

    assert_eq!(vec_original.len(), 2);
    assert_eq!(vec_appended.len(), 3);
    assert_eq!(vector_original.len(), 2);
    assert_eq!(vector_appended.len(), 3);
}
