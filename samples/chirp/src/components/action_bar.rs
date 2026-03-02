use leptos::prelude::*;
use leptos_router::components::A;

use crate::server::social::{toggle_like, toggle_rechirp};

/// ActionBar — いいね・リチャープ・リプライのインタラクション
///
/// このコンポーネントには Rust × フロントエンドの設計的緊張が凝縮されている:
///
/// 1. `post_id: String` は `Copy` でないため、複数の `move` クロージャに渡すには `.clone()` が必要
///    → Signal (Copy) と String (Clone) の世界の違い
///
/// 2. `on:click` は同期クロージャだが、`toggle_like` は非同期サーバー関数
///    → `spawn_local` で同期の世界から非同期の世界へ橋を架ける
///
/// 3. 楽観的更新は「UIを即座に変える→サーバーに問い合わせる→失敗したら元に戻す」
///    → フロントエンドの即時性と Rust の Result 型による正確性の交渉
#[component]
pub fn ActionBar(
    post_id: String,
    reply_count: i32,
    like_count: i32,
    rechirp_count: i32,
    liked: bool,
    rechirped: bool,
) -> impl IntoView {
    // --- Signal は Copy: 何度でもクロージャに渡せる ---
    let (is_liked, set_liked) = signal(liked);
    let (local_like_count, set_like_count) = signal(like_count);
    let (is_rechirped, set_rechirped) = signal(rechirped);
    let (local_rechirp_count, set_rechirp_count) = signal(rechirp_count);

    // --- String は Copy でない: 各クロージャ用にクローンが必要 ---
    // JavaScript なら変数を参照するだけでいい。Rust では「誰がこの文字列を所有するか」を
    // 明示的に決めなければならない。この 3 行の clone は、所有権モデルの代償であり、
    // 同時に「この値がどこで使われるか」を可視化する手段でもある。
    let post_id_for_like = post_id.clone();
    let post_id_for_rechirp = post_id.clone();
    let post_id_for_reply = post_id;

    // --- いいねハンドラ: 楽観的更新 + ロールバック ---
    let on_like = move |_| {
        // get_untracked(): 値を読むが購読はしない
        // ここで .get() を使うと、このクロージャ自体が is_liked の変更で再実行される
        // → 意図しない無限ループの原因になりうる
        let was_liked = is_liked.get_untracked();

        // 楽観的更新: サーバーの応答を待たずにUIを即座に変更する
        // フロントエンドの「ユーザーを待たせない」要請がここに表れている
        set_liked.set(!was_liked);
        set_like_count.update(|c| {
            if was_liked {
                *c -= 1;
            } else {
                *c += 1;
            }
        });

        // --- sync → async ブリッジ ---
        // on:click は同期クロージャ `Fn(MouseEvent)` を期待する。
        // しかし toggle_like はサーバー関数で async。
        // spawn_local は「この非同期タスクをブラウザのイベントループに投げる」ことで
        // 同期の世界と非同期の世界を橋渡しする。
        //
        // JavaScript なら `onClick={async () => { await fetch(...) }}` と書ける。
        // Rust の型システムは sync と async を厳密に区別するため、この橋が必要になる。
        let pid = post_id_for_like.clone();
        leptos::task::spawn_local(async move {
            // サーバー呼び出し: ネットワーク越しの Result<bool, ServerFnError>
            if toggle_like(pid).await.is_err() {
                // ロールバック: サーバーが失敗したらUIを元に戻す
                // Rust の Result 型が「失敗は常にありうる」ことを型レベルで表現し、
                // コンパイラがその処理を強制する。JavaScript では try/catch の書き忘れが
                // 静かにバグを生むが、Rust では Result を処理しなければコンパイルが通らない。
                set_liked.set(was_liked);
                set_like_count.update(|c| {
                    if was_liked {
                        *c += 1;
                    } else {
                        *c -= 1;
                    }
                });
            }
        });
    };

    // --- リチャープハンドラ: 同じパターン ---
    let on_rechirp = move |_| {
        let was_rechirped = is_rechirped.get_untracked();

        set_rechirped.set(!was_rechirped);
        set_rechirp_count.update(|c| {
            if was_rechirped {
                *c -= 1;
            } else {
                *c += 1;
            }
        });

        let pid = post_id_for_rechirp.clone();
        leptos::task::spawn_local(async move {
            if toggle_rechirp(pid).await.is_err() {
                set_rechirped.set(was_rechirped);
                set_rechirp_count.update(|c| {
                    if was_rechirped {
                        *c += 1;
                    } else {
                        *c -= 1;
                    }
                });
            }
        });
    };

    view! {
        <div class="flex items-center gap-6 mt-2 text-gray-500 text-sm">
            // リプライ: ページ遷移
            <A
                href=format!("/post/{}", post_id_for_reply)
                attr:class="flex items-center gap-1 hover:text-blue-400 transition-colors"
            >
                <span>"💬"</span>
                <span>{reply_count}</span>
            </A>

            // リチャープ: 楽観的トグル
            <button
                class=move || {
                    if is_rechirped.get() {
                        "flex items-center gap-1 text-green-500 transition-colors"
                    } else {
                        "flex items-center gap-1 hover:text-green-500 transition-colors"
                    }
                }
                on:click=on_rechirp
            >
                <span>"🔁"</span>
                <span>{move || local_rechirp_count.get()}</span>
            </button>

            // いいね: 楽観的トグル
            <button
                class=move || {
                    if is_liked.get() {
                        "flex items-center gap-1 text-pink-600 transition-colors"
                    } else {
                        "flex items-center gap-1 hover:text-pink-600 transition-colors"
                    }
                }
                on:click=on_like
            >
                <span>{move || if is_liked.get() { "❤️" } else { "🤍" }}</span>
                <span>{move || local_like_count.get()}</span>
            </button>
        </div>
    }
}
