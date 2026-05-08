//! `#[non_exhaustive]` は「将来プランが増えるかもしれない」ことを型に刻む。
//! downstream 側は wildcard を書き、struct literal ではなく constructor を使う。

#![allow(
    clippy::panic,
    clippy::print_stdout,
    reason = "examples print their narrative output and may use unreachable demo branches"
)]

use rust_types_as_walls::api_evolution::{BillingPlan, CheckoutSummary, describe};

fn ui_label(plan: BillingPlan) -> &'static str {
    match plan {
        BillingPlan::Free => "無料プラン",
        BillingPlan::Pro => "Pro プラン",
        _ => "将来追加されるプラン",
    }
}

fn main() {
    let summary = CheckoutSummary::new("future@example.com", BillingPlan::Pro);
    println!("{}", describe(&summary));
    println!("UI 表示名: {}", ui_label(summary.plan));

    // 外部 crate では次のようなコードは書けない:
    // - `match` を Free / Pro だけで exhaust させる
    // - `CheckoutSummary { ... }` の struct literal で直接生成する
    // どちらも将来の拡張余地を壊さないための「壁」になる。
}
