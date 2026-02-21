use leptos::prelude::*;
use leptos_router::hooks::use_params_map;

use crate::components::post_card::PostCard;
use crate::components::post_composer::PostComposer;
use crate::components::post_list::PostList;

#[component]
pub fn PostDetailPage() -> impl IntoView {
    let params = use_params_map();
    let post_id = move || params.get().get("id").unwrap_or_default();

    let post_detail = Resource::new(post_id, |id| async move { get_post(id).await });

    view! {
        <div class="max-w-[600px] mx-auto border-x border-gray-800 min-h-screen">
            <header class="sticky top-0 z-10 bg-black/80 backdrop-blur-md border-b border-gray-800">
                <h1 class="text-xl font-bold p-4">"投稿"</h1>
            </header>

            <Suspense fallback=move || {
                view! { <div class="p-4 text-gray-500">"読み込み中..."</div> }
            }>
                {move || {
                    post_detail
                        .get()
                        .map(|result| {
                            match result {
                                Ok(detail) => {
                                    let reply_id = detail.post.id.to_string();
                                    view! {
                                        <div>
                                            {detail
                                                .parent
                                                .map(|p| view! { <PostCard post=*p /> })}
                                            <PostCard post=detail.post />
                                            <PostComposer reply_to_id=Some(reply_id) />
                                            <PostList posts=detail.replies />
                                        </div>
                                    }
                                        .into_any()
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
        </div>
    }
}

use crate::server::posts::get_post;
