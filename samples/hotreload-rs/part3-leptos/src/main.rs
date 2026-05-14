//! Leptos 0.8.17 ホットリロード検証アプリ（CSR モード）。
//!
//! # 実行方法
//!
//! ## trunk を使った開発（CSR モード、ファイル監視 + フルリビルド）
//! ```sh
//! cd part3-leptos
//! trunk serve --open
//! ```
//!
//! ## Subsecond ホットパッチ（dx CLI 経由、実験的）
//! ```sh
//! cd part3-leptos
//! RUSTFLAGS="" dx serve --hot-patch --platform web --features subsecond
//! ```
//!
//! # 検証シナリオ
//!
//! ## trunk serve（フルリビルド方式）
//! - [ ] Rust ソース変更 → WASM フルリビルド + 自動リロード
//! - [ ] CSS 変更 → アセットのみ再バンドル（Rust リコンパイルなし）
//!
//! ## Subsecond ホットパッチ（dx serve --hot-patch）
//! - [ ] view テンプレートのテキスト変更 → パッチ反映されるか
//! - [ ] compute() のロジック変更 → パッチ反映されるか
//! - [ ] 新しい変数の追加 → フルリビルドが走るか
//! - [ ] 状態（signal）はリセットされるか保持されるか

use leptos::prelude::*;

fn main() {
    console_error_panic_hook::set_once();

    // Subsecond feature 有効時: dx devserver の WebSocket に接続し、
    // ホットパッチメッセージを受信する。クロージャで包むことでトップレベル
    // コンポーネント自体をリアクティブにし、パッチ適用時に再呼び出し可能にする。
    // trunk serve 時（subsecond feature 無効時）はコンパイルされない。
    #[cfg(feature = "subsecond")]
    {
        leptos::subsecond::connect_to_hot_patch_messages();
        mount_to_body(|| App);
    }

    #[cfg(not(feature = "subsecond"))]
    mount_to_body(App);
}

/// メインアプリコンポーネント。
#[component]
fn App() -> impl IntoView {
    view! {
        <div class="container">
            <h1>"Leptos 0.8 Hot-Reload Demo"</h1>

            // --- Section 1: カウンター ---
            <CounterSection />

            // --- Section 2: TODO リスト（For コンポーネント検証） ---
            <TodoSection />

            // --- Section 3: 条件付きレンダリング ---
            <ConditionalSection />

            // --- Section 4: 計算ロジック（Subsecond 検証） ---
            <ComputeSection />

            <p class="note">
                "このアプリは Leptos のホットリロード検証用です。"
                " view の変更は cargo leptos watch --hot-reload で即時反映、"
                " Rust コードの変更は Subsecond 統合（実験的）で反映されます。"
            </p>
        </div>
    }
}

// =============================================================================
// Section 1: カウンター
// =============================================================================

/// カウンターコンポーネント。
///
/// view パッチ検証: ボタンのラベルやレイアウトを変更して即時反映されるか確認。
/// Subsecond 検証: increment のロジックを変更して反映されるか確認。
#[component]
fn CounterSection() -> impl IntoView {
    let (count, set_count) = signal(0i64);

    // Subsecond 検証: この計算を変更してみる（例: * 2 → * 3）
    let doubled = move || count.get() * 2;

    view! {
        <div class="card">
            <h2>"Counter"</h2>
            <div class="counter-value">{count}</div>
            <p>"Doubled: " {doubled}</p>
            <div class="btn-group">
                // view パッチ検証: ボタンのラベルを変えてみる
                <button on:click=move |_| set_count.update(|n| *n += 1)>"+1"</button>
                <button on:click=move |_| set_count.update(|n| *n -= 1)>"-1"</button>
                <button on:click=move |_| set_count.set(0)>"Reset"</button>
            </div>
        </div>
    }
}

// =============================================================================
// Section 2: TODO リスト
// =============================================================================

/// TODO リストコンポーネント。
///
/// view パッチ検証: リストアイテムの表示フォーマットを変更して即時反映されるか確認。
#[component]
fn TodoSection() -> impl IntoView {
    let (items, set_items) = signal(vec![
        "hot-lib-reloader を試す".to_string(),
        "Dioxus RSX ホットリロードを検証".to_string(),
        "Leptos view パッチを検証".to_string(),
    ]);
    let (input, set_input) = signal(String::new());

    let add_item = move |_| {
        let value = input.get_untracked();
        if !value.is_empty() {
            set_items.update(|list| list.push(value));
            set_input.set(String::new());
        }
    };

    view! {
        <div class="card">
            <h2>"TODO List"</h2>

            <div class="btn-group">
                <input
                    type="text"
                    prop:value=input
                    placeholder="New item..."
                    on:input=move |e| set_input.set(event_target_value(&e))
                />
                <button on:click=add_item>"Add"</button>
            </div>

            <ul class="item-list">
                // view パッチ検証: この表示フォーマットを変えてみる
                // 例: "{i}. {item}" → "✓ {item}"
                <For
                    each=move || {
                        items.get().into_iter().enumerate().collect::<Vec<_>>()
                    }
                    key=|item| item.0
                    children=move |(i, item)| {
                        view! {
                            <li>{format!("{}. {}", i + 1, item)}</li>
                        }
                    }
                />
            </ul>

            <Show when=move || items.get().is_empty()>
                <p class="note">"No items yet."</p>
            </Show>
        </div>
    }
}

// =============================================================================
// Section 3: 条件付きレンダリング
// =============================================================================

/// 条件分岐コンポーネント。
///
/// view パッチ検証: Show / match 内のテキストを変更して即時反映されるか確認。
#[component]
fn ConditionalSection() -> impl IntoView {
    let (show_details, set_show_details) = signal(false);
    let (selected_tab, set_selected_tab) = signal(0u8);

    let toggle_label = move || {
        if show_details.get() {
            "Hide Details"
        } else {
            "Show Details"
        }
    };

    view! {
        <div class="card">
            <h2>"Conditional Rendering"</h2>

            <button on:click=move |_| set_show_details.update(|v| *v = !*v)>
                {toggle_label}
            </button>

            <Show when=move || show_details.get()>
                <div>
                    <p>"This section is conditionally rendered."</p>
                    <p>"Try changing this text while the app is running!"</p>

                    <div class="btn-group">
                        <button on:click=move |_| set_selected_tab.set(0)>"Tab A"</button>
                        <button on:click=move |_| set_selected_tab.set(1)>"Tab B"</button>
                        <button on:click=move |_| set_selected_tab.set(2)>"Tab C"</button>
                    </div>

                    // view パッチ検証: タブの内容を変更してみる
                    {move || match selected_tab.get() {
                        0 => view! { <p>"Content for Tab A — try changing me!"</p> }.into_any(),
                        1 => view! { <p>"Content for Tab B — or change me!"</p> }.into_any(),
                        _ => view! { <p>"Content for Tab C — change us all!"</p> }.into_any(),
                    }}
                </div>
            </Show>
        </div>
    }
}

// =============================================================================
// Section 4: 計算ロジック — Subsecond 検証
// =============================================================================

/// 計算ロジックコンポーネント。Subsecond 統合時にロジック変更が反映されるか検証。
#[component]
fn ComputeSection() -> impl IntoView {
    let (input_num, set_input_num) = signal(7u64);

    // Subsecond 検証: compute() の中身を変更してみる
    let result = move || compute(input_num.get());

    view! {
        <div class="card">
            <h2>"Compute (Subsecond Test)"</h2>
            <p>"Input: " {input_num}</p>
            <p>"Result: " {result}</p>
            <div class="btn-group">
                <button on:click=move |_| set_input_num.update(|n| *n += 1)>"+1"</button>
                <button on:click=move |_| set_input_num.update(|n| {
                    if *n > 0 { *n -= 1; }
                })>"-1"</button>
            </div>
            <p class="note">
                "Change the compute() function and see if Subsecond reflects it."
            </p>
        </div>
    }
}

/// ビジネスロジック関数。Subsecond で変更が反映されるかの検証対象。
///
/// 試してみること:
/// 1. `n * n` → `n * n * n`
/// 2. `+ 42` を追加
/// 3. 全く別の計算に変更
fn compute(n: u64) -> u64 {
    n * n
}
