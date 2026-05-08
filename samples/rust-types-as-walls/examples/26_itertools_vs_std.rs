//! `itertools` の iterator 拡張を std の手書き処理と対比する。
//! `chunk_by`, `tuple_windows`, `cartesian_product`, `intersperse` は
//! 「集める」「ずらす」「組み合わせる」をパイプラインのまま書ける。

#![allow(
    clippy::panic,
    clippy::print_stdout,
    reason = "examples print their narrative output and may use unreachable demo branches"
)]

use itertools::Itertools;

fn grouped_with_std(records: &[(&str, &str)]) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current_customer = "";
    let mut current_skus = Vec::new();

    for &(customer, sku) in records {
        if current_customer.is_empty() {
            current_customer = customer;
        }

        if customer != current_customer {
            lines.push(format!("{current_customer}: {}", current_skus.join(", ")));
            current_customer = customer;
            current_skus = Vec::new();
        }

        current_skus.push(sku.to_owned());
    }

    if !current_customer.is_empty() {
        lines.push(format!("{current_customer}: {}", current_skus.join(", ")));
    }

    lines
}

fn grouped_with_itertools(records: &[(&str, &str)]) -> Vec<String> {
    let grouped_records = records.iter().chunk_by(|(customer, _)| *customer);

    let mut lines = Vec::new();
    for (customer, chunk) in &grouped_records {
        let skus = chunk.map(|(_, sku)| (*sku).to_owned()).collect::<Vec<_>>();
        lines.push(format!("{customer}: {}", skus.join(", ")));
    }

    lines
}

fn tuple_windows_with_std(prices: &[u64]) -> Vec<(u64, u64)> {
    prices
        .windows(2)
        .map(|window| (window[0], window[1]))
        .collect()
}

fn tuple_windows_with_itertools(prices: &[u64]) -> Vec<(u64, u64)> {
    prices.iter().copied().tuple_windows().collect()
}

fn cartesian_with_std(sizes: &[&str], colors: &[&str]) -> Vec<String> {
    let mut combinations = Vec::new();

    for size in sizes {
        for color in colors {
            combinations.push(format!("{size}-{color}"));
        }
    }

    combinations
}

fn cartesian_with_itertools(sizes: &[&str], colors: &[&str]) -> Vec<String> {
    sizes
        .iter()
        .cartesian_product(colors.iter())
        .map(|(size, color)| format!("{size}-{color}"))
        .collect()
}

fn banner_with_std(stages: &[&str]) -> String {
    stages.join(" -> ")
}

fn banner_with_itertools(stages: &[&str]) -> String {
    Itertools::intersperse(
        stages.iter().map(|stage| (*stage).to_owned()),
        " -> ".to_owned(),
    )
    .fold(String::new(), |mut banner, part| {
        banner.push_str(&part);
        banner
    })
}

fn main() {
    let grouped_records = [
        ("alice", "BOOK-001"),
        ("alice", "PEN-001"),
        ("bob", "BAG-001"),
        ("bob", "STICKER-001"),
    ];
    let prices = [1_500, 1_800, 2_100];
    let sizes = ["S", "M"];
    let colors = ["black", "ivory"];
    let stages = ["receive", "validate", "price", "charge"];

    let grouped_std = grouped_with_std(&grouped_records);
    let grouped_itertools = grouped_with_itertools(&grouped_records);
    let windows_std = tuple_windows_with_std(&prices);
    let windows_itertools = tuple_windows_with_itertools(&prices);
    let combinations_std = cartesian_with_std(&sizes, &colors);
    let combinations_itertools = cartesian_with_itertools(&sizes, &colors);
    let banner_std = banner_with_std(&stages);
    let banner_itertools = banner_with_itertools(&stages);

    println!("chunk_by: {grouped_itertools:?}");
    println!("tuple_windows: {windows_itertools:?}");
    println!("cartesian_product: {combinations_itertools:?}");
    println!("intersperse: {banner_itertools}");

    assert_eq!(grouped_std, grouped_itertools);
    assert_eq!(windows_std, windows_itertools);
    assert_eq!(combinations_std, combinations_itertools);
    assert_eq!(banner_std, banner_itertools);
}
