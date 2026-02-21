use leptos::prelude::*;
use leptos_router::components::A;
use leptos_router::hooks::use_query_map;

use crate::components::post_list::PostList;
use crate::components::user_avatar::UserAvatar;

#[component]
pub fn SearchPage() -> impl IntoView {
    let query_map = use_query_map();
    let search_query = move || query_map.get().get("q").unwrap_or_default();

    let post_results = Resource::new(search_query, |q| async move {
        if q.is_empty() {
            Ok(vec![])
        } else {
            search_posts(q, None).await
        }
    });

    let user_results = Resource::new(search_query, |q| async move {
        if q.is_empty() {
            Ok(vec![])
        } else {
            search_users(q, Some(5)).await
        }
    });

    view! {
        <div class="max-w-[600px] mx-auto border-x border-gray-800 min-h-screen">
            <header class="sticky top-0 z-10 bg-black/80 backdrop-blur-md border-b border-gray-800 p-4">
                <form action="/search" method="get">
                    <input
                        type="text"
                        name="q"
                        value=search_query
                        placeholder="検索..."
                        class="w-full px-4 py-2 bg-gray-900 border border-gray-700 rounded-full focus:outline-none focus:border-sky-500"
                    />
                </form>
            </header>

            <Suspense fallback=move || {
                view! { <div class="p-4 text-gray-500">"検索中..."</div> }
            }>
                {move || {
                    user_results
                        .get()
                        .map(|result| {
                            match result {
                                Ok(users) if !users.is_empty() => {
                                    view! {
                                        <div class="border-b border-gray-800">
                                            <h2 class="text-lg font-bold p-4">"ユーザー"</h2>
                                            {users
                                                .into_iter()
                                                .map(|u| {
                                                    let href = format!("/user/{}", u.username);
                                                    let at_name = format!("@{}", u.username);
                                                    let display = u.display_name.clone();
                                                    view! {
                                                        <A
                                                            href=href
                                                            attr:class="flex items-center gap-3 p-4 hover:bg-gray-900/50"
                                                        >
                                                            <UserAvatar url=u.avatar_url size=40 />
                                                            <div>
                                                                <p class="font-bold">{display}</p>
                                                                <p class="text-gray-500">{at_name}</p>
                                                            </div>
                                                        </A>
                                                    }
                                                })
                                                .collect::<Vec<_>>()}
                                        </div>
                                    }
                                        .into_any()
                                }
                                _ => view! { <div></div> }.into_any(),
                            }
                        })
                }}
            </Suspense>

            <Suspense fallback=move || {
                view! { <div class="p-4 text-gray-500">"投稿を検索中..."</div> }
            }>
                {move || {
                    post_results
                        .get()
                        .map(|result| {
                            match result {
                                Ok(posts) => view! { <PostList posts=posts /> }.into_any(),
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
        </div>
    }
}

use crate::server::search::{search_posts, search_users};
