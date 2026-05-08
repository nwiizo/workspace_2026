//! `rayon` で純粋 map-reduce を並列化する。
//! 各注文の小計計算を副作用なしの関数に閉じ込めておけば、
//! `iter()` を `par_iter()` に差し替えても意味を崩しにくい。

#![allow(
    clippy::panic,
    clippy::print_stdout,
    reason = "examples print their narrative output and may use unreachable demo branches"
)]

use rayon::prelude::*;

#[derive(Debug, Clone)]
struct OrderLine {
    quantity: u32,
    unit_price: u64,
}

#[derive(Debug, Clone)]
struct Order {
    id: &'static str,
    lines: Vec<OrderLine>,
}

fn order_total(order: &Order) -> u64 {
    order
        .lines
        .iter()
        .map(|line| line.unit_price * u64::from(line.quantity))
        .sum()
}

fn total_serial(orders: &[Order]) -> u64 {
    orders.iter().map(order_total).sum()
}

fn total_parallel(orders: &[Order]) -> u64 {
    orders
        .par_iter()
        .map(order_total)
        .reduce(|| 0, |left_total, right_total| left_total + right_total)
}

fn main() {
    let orders = vec![
        Order {
            id: "order_1",
            lines: vec![
                OrderLine {
                    quantity: 2,
                    unit_price: 1_500,
                },
                OrderLine {
                    quantity: 3,
                    unit_price: 300,
                },
            ],
        },
        Order {
            id: "order_2",
            lines: vec![
                OrderLine {
                    quantity: 1,
                    unit_price: 4_800,
                },
                OrderLine {
                    quantity: 5,
                    unit_price: 200,
                },
            ],
        },
    ];

    let serial_total = total_serial(&orders);
    let parallel_total = total_parallel(&orders);
    let per_order_parallel = orders
        .par_iter()
        .map(|order| format!("{}={}円", order.id, order_total(order)))
        .collect::<Vec<_>>();

    println!("serial total={serial_total}");
    println!("parallel total={parallel_total}");
    println!("parallel per-order={per_order_parallel:?}");

    assert_eq!(serial_total, parallel_total);
}
