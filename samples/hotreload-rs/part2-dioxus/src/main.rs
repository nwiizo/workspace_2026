//! Dioxus 0.7.4 ホットリロード検証アプリ。
//!
//! # 実行方法
//!
//! ## RSX ホットリロード（デフォルト）
//! ```sh
//! cd part2-dioxus
//! dx serve
//! ```
//!
//! ## Rust ホットパッチ（Subsecond）
//! ```sh
//! cd part2-dioxus
//! dx serve --hotpatch
//! ```
//!
//! # 検証シナリオ
//!
//! ## RSX ホットリロード（dx serve）
//! - [ ] 要素の追加・削除 → 即時反映されるか
//! - [ ] 属性値の変更（文字列、数値、bool） → 即時反映されるか
//! - [ ] フォーマット文字列内の変数移動 → 即時反映されるか
//! - [ ] for ループ内の要素変更 → 即時反映されるか
//! - [ ] if 条件ブロック内の要素変更 → 即時反映されるか
//!
//! ## Subsecond ホットパッチ（dx serve --hotpatch）
//! - [ ] RSX 外のロジック変更（関数本体）→ 反映されるか
//! - [ ] hooks 内の計算変更 → 反映されるか
//! - [ ] 新しい変数・式の追加 → フルリビルドが走るか
//!
//! ## CSS ホットリロード
//! - [ ] assets/main.css の変更 → リロードなしで反映されるか

use dioxus::prelude::*;

fn main() {
    dioxus::launch(app);
}

/// メインアプリコンポーネント。
fn app() -> Element {
    rsx! {
        div { class: "container",
            h1 { "Dioxus 0.7 Hot-Reload Demo" }

            // --- Section 1: カウンター（状態管理 + RSX パッチ検証） ---
            CounterSection {}

            // --- Section 2: TODO リスト（for ループ + 条件分岐検証） ---
            TodoSection {}

            // --- Section 3: 条件付きレンダリング（if ブロック検証） ---
            ConditionalSection {}

            // --- Section 4: Subsecond 検証用（ロジック変更の反映） ---
            ComputeSection {}

            p { class: "note",
                "このアプリは Dioxus のホットリロード検証用です。"
                " RSX の変更は dx serve で即時反映、"
                " Rust コードの変更は dx serve --hotpatch で反映されます。"
            }
        }
    }
}

// =============================================================================
// Section 1: カウンター — 基本的な状態管理と RSX パッチ
// =============================================================================

/// カウンターコンポーネント。
///
/// RSX 検証: ボタンのラベルやレイアウトを変えて即時反映されるか確認。
/// Subsecond 検証: increment のロジック（+1 → +5 など）を変えて反映されるか確認。
#[component]
fn CounterSection() -> Element {
    let mut count = use_signal(|| 0i64);

    // Subsecond 検証: この計算ロジックを変更してみる（例: * 2 → * 3）
    let doubled = count() * 2;

    rsx! {
        div { class: "card",
            h2 { "Counter" }
            div { class: "counter-value", "{count}" }
            p { "Doubled: {doubled}" }
            div { class: "btn-group",
                // RSX 検証: ボタンのラベルや数を変えてみる
                button { onclick: move |_| count += 1, "+1" }
                button { onclick: move |_| count -= 1, "-1" }
                button { onclick: move |_| count.set(0), "Reset" }
            }
        }
    }
}

// =============================================================================
// Section 2: TODO リスト — for ループ内の RSX パッチ
// =============================================================================

/// TODO リストコンポーネント。
///
/// RSX 検証: リストアイテムの表示フォーマットを変えて即時反映されるか確認。
/// for ループ内の要素追加・削除も検証。
#[component]
fn TodoSection() -> Element {
    let mut items = use_signal(|| {
        vec![
            "hot-lib-reloader を試す".to_string(),
            "Dioxus RSX ホットリロードを検証".to_string(),
            "Leptos view パッチを検証".to_string(),
        ]
    });
    let mut input = use_signal(String::new);

    rsx! {
        div { class: "card",
            h2 { "TODO List" }

            div { class: "btn-group",
                input {
                    value: "{input}",
                    placeholder: "New item...",
                    oninput: move |e| input.set(e.value()),
                }
                button {
                    onclick: move |_| {
                        let value = input.peek().clone();
                        if !value.is_empty() {
                            items.push(value);
                            input.set(String::new());
                        }
                    },
                    "Add"
                }
            }

            ul { class: "item-list",
                // RSX 検証: この for ループ内の表示を変更してみる
                // 例: "- {item}" → "✓ {item}" や番号付きに変更
                for (i, item) in items.read().iter().enumerate() {
                    li { key: "{i}", "{i + 1}. {item}" }
                }
            }

            if items.read().is_empty() {
                p { class: "note", "No items yet." }
            }
        }
    }
}

// =============================================================================
// Section 3: 条件付きレンダリング — if ブロックの RSX パッチ
// =============================================================================

/// 条件分岐コンポーネント。
///
/// RSX 検証: if/else 内のテキストや構造を変更して即時反映されるか確認。
#[component]
fn ConditionalSection() -> Element {
    let mut show_details = use_signal(|| false);
    let mut selected_tab = use_signal(|| 0u8);

    rsx! {
        div { class: "card",
            h2 { "Conditional Rendering" }

            button {
                onclick: move |_| show_details.toggle(),
                if show_details() { "Hide Details" } else { "Show Details" }
            }

            // RSX 検証: この条件ブロック内の内容を変更してみる
            if show_details() {
                div {
                    p { "This section is conditionally rendered." }
                    p { "Try changing this text while the app is running!" }

                    div { class: "btn-group",
                        button {
                            onclick: move |_| selected_tab.set(0),
                            "Tab A"
                        }
                        button {
                            onclick: move |_| selected_tab.set(1),
                            "Tab B"
                        }
                        button {
                            onclick: move |_| selected_tab.set(2),
                            "Tab C"
                        }
                    }

                    // RSX 検証: タブの内容を変更してみる
                    match selected_tab() {
                        0 => rsx! { p { "Content for Tab A — try changing me!" } },
                        1 => rsx! { p { "Content for Tab B — or change me!" } },
                        _ => rsx! { p { "Content for Tab C — change us all!" } },
                    }
                }
            }
        }
    }
}

// =============================================================================
// Section 4: 計算ロジック — Subsecond（ホットパッチ）検証
// =============================================================================

/// 計算ロジックコンポーネント。
///
/// Subsecond 検証: この関数群のロジックを変更して、`dx serve --hotpatch` で
/// 反映されるか確認する。RSX だけでなく Rust コード本体が対象。
#[component]
fn ComputeSection() -> Element {
    let mut input_num = use_signal(|| 7u64);

    // Subsecond 検証: この計算ロジックを変更してみる
    let result = compute(input_num());

    rsx! {
        div { class: "card",
            h2 { "Compute (Subsecond Test)" }
            p { "Input: {input_num}" }
            p { "Result: {result}" }
            div { class: "btn-group",
                button { onclick: move |_| input_num += 1, "+1" }
                button { onclick: move |_| {
                    let v = input_num();
                    if v > 0 { input_num.set(v - 1); }
                }, "-1" }
            }
            p { class: "note",
                "Change the compute() function logic and see if --hotpatch reflects it."
            }
        }
    }
}

/// ビジネスロジック関数。Subsecond で変更が反映されるかの検証対象。
///
/// 試してみること:
/// 1. `n * n` → `n * n * n`（二乗 → 三乗）
/// 2. `+ 42` を追加
/// 3. 全く別の計算に変更
fn compute(n: u64) -> u64 {
    n * n
}
