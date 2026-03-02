use leptos::prelude::*;

use crate::components::post_card::PostCard;
use crate::server::timeline::get_public_timeline;

#[component]
pub fn ExplorePage() -> impl IntoView {
    let posts = RwSignal::new(Vec::new());
    let cursor = RwSignal::new(None::<String>);
    let is_loading = RwSignal::new(false);
    let has_more = RwSignal::new(true);

    let initial_load = Resource::new(|| (), |_| async { get_public_timeline(None, None).await });

    let load_more = move |_| {
        if is_loading.get_untracked() || !has_more.get_untracked() {
            return;
        }
        is_loading.set(true);
        let current_cursor = cursor.get_untracked();

        leptos::task::spawn_local(async move {
            match get_public_timeline(current_cursor, None).await {
                Ok(new_posts) => {
                    if new_posts.len() < 20 {
                        has_more.set(false);
                    }
                    if let Some(last) = new_posts.last() {
                        cursor.set(Some(last.id.to_string()));
                    }
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
                <h1 class="text-xl font-bold p-4">"探索"</h1>
            </header>

            <Suspense fallback=move || {
                view! { <div class="p-4 text-gray-500">"読み込み中..."</div> }
            }>
                {move || {
                    initial_load
                        .get()
                        .map(|result| {
                            match result {
                                Ok(initial_posts) => {
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
