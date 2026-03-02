use leptos::prelude::*;

use crate::components::post_card::PostCard;
use crate::components::post_composer::PostComposer;
use crate::server::auth::get_current_user;
use crate::server::timeline::get_home_timeline;

/// HomePage — カーソルベースの「もっと見る」ページネーション
///
/// ここには Rust の型システムがフロントエンドの状態遷移をどう表現するかが表れている:
///
/// - `RwSignal<Vec<PostWithMeta>>`: 蓄積される投稿リスト。Rust では Vec の所有権が明確
/// - `RwSignal<Option<String>>`: カーソル状態。None = 初回、Some = 続きあり
/// - `RwSignal<bool>`: ローディング状態
///
/// JavaScript なら `let posts = []; let cursor = null;` で済む。
/// Rust では型が状態の「形」を規定し、不正な状態遷移をコンパイル時に防ぐ。
#[component]
pub fn HomePage() -> impl IntoView {
    let current_user = Resource::new(|| (), |_| async { get_current_user().await });

    // --- 蓄積型の状態管理 ---
    // Resource は「一度の非同期取得」に向いているが、ページネーションのように
    // 「前の結果を保持しつつ新しいデータを追加する」パターンには Signal が必要。
    //
    // RwSignal は ReadSignal + WriteSignal を一つにまとめた型。
    // コンポーネント内で読み書き両方が必要な場合に使う。
    let posts = RwSignal::new(Vec::new());
    let cursor = RwSignal::new(None::<String>);
    let is_loading = RwSignal::new(false);
    let has_more = RwSignal::new(true);

    // --- 初回ロード ---
    // Resource ではなく spawn_local で即座にデータ取得を開始する。
    // Resource だと「データをリアクティブに管理する」意味論になるが、
    // ここでは「Signal に追記していく」方が自然なため。
    let initial_load = Resource::new(|| (), |_| async { get_home_timeline(None, None).await });

    // --- 「もっと見る」ハンドラ ---
    let load_more = move |_| {
        if is_loading.get_untracked() || !has_more.get_untracked() {
            return;
        }

        is_loading.set(true);

        // cursor の現在値をクローン — Option<String> は Copy でないため
        let current_cursor = cursor.get_untracked();

        leptos::task::spawn_local(async move {
            match get_home_timeline(current_cursor, None).await {
                Ok(new_posts) => {
                    if new_posts.len() < 20 {
                        has_more.set(false);
                    }
                    // 次のカーソル = 最後の投稿のID
                    if let Some(last) = new_posts.last() {
                        cursor.set(Some(last.id.to_string()));
                    }
                    // 既存の投稿に追記 — Vec の所有権は RwSignal が管理
                    posts.update(|p| p.extend(new_posts));
                }
                Err(_) => {
                    has_more.set(false);
                }
            }
            is_loading.set(false);
        });
    };

    view! {
        <div class="max-w-[600px] mx-auto border-x border-gray-800 min-h-screen">
            <header class="sticky top-0 z-10 bg-black/80 backdrop-blur-md border-b border-gray-800">
                <h1 class="text-xl font-bold p-4">"ホーム"</h1>
            </header>

            <Suspense fallback=move || {
                view! { <div class="p-4 text-gray-500">"読み込み中..."</div> }
            }>
                {move || {
                    let user = current_user.get().and_then(|r| r.ok()).flatten();
                    let avatar_url = user.as_ref().and_then(|u| u.avatar_url.clone());
                    view! { <PostComposer avatar_url=avatar_url /> }
                }}
            </Suspense>

            // --- 初回ロード: Resource + Suspense ---
            <Suspense fallback=move || {
                view! { <div class="p-4 text-gray-500">"タイムラインを読み込み中..."</div> }
            }>
                {move || {
                    initial_load
                        .get()
                        .map(|result| {
                            match result {
                                Ok(initial_posts) => {
                                    // 初回データを Signal にセット
                                    if posts.get_untracked().is_empty() {
                                        if initial_posts.len() < 20 {
                                            has_more.set(false);
                                        }
                                        if let Some(last) = initial_posts.last() {
                                            cursor.set(Some(last.id.to_string()));
                                        }
                                        posts.set(initial_posts);
                                    }
                                    view! { <div /> }.into_any()
                                }
                                Err(e) => {
                                    view! {
                                        <div class="p-4 text-red-500">
                                            {format!("エラー: {e}")}
                                        </div>
                                    }
                                        .into_any()
                                }
                            }
                        })
                }}
            </Suspense>

            // --- 蓄積された投稿リスト ---
            // move || posts.get() — Signal の変更で自動更新される
            {move || {
                let current_posts = posts.get();
                if current_posts.is_empty() {
                    view! {
                        <div class="p-8 text-center text-gray-500">
                            "まだ投稿がありません"
                        </div>
                    }
                        .into_any()
                } else {
                    view! {
                        <div>
                            {current_posts
                                .into_iter()
                                .map(|post| view! { <PostCard post=post /> })
                                .collect::<Vec<_>>()}
                        </div>
                    }
                        .into_any()
                }
            }}

            // --- 「もっと見る」ボタン ---
            // has_more と is_loading の組み合わせで表示を制御
            {move || {
                if has_more.get() {
                    view! {
                        <div class="p-4 text-center border-t border-gray-800">
                            <button
                                class="text-sky-500 hover:text-sky-400 font-bold transition-colors disabled:opacity-50"
                                on:click=load_more
                                disabled=move || is_loading.get()
                            >
                                {move || {
                                    if is_loading.get() {
                                        "読み込み中..."
                                    } else {
                                        "もっと見る"
                                    }
                                }}
                            </button>
                        </div>
                    }
                        .into_any()
                } else {
                    view! { <div /> }.into_any()
                }
            }}
        </div>
    }
}
