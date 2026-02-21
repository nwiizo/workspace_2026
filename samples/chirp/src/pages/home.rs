use leptos::prelude::*;

use crate::components::post_composer::PostComposer;
use crate::components::post_list::PostList;

#[component]
pub fn HomePage() -> impl IntoView {
    let current_user = Resource::new(|| (), |_| async { get_current_user().await });
    let timeline = Resource::new(|| (), |_| async { get_home_timeline(None, None).await });

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

            <Suspense fallback=move || {
                view! { <div class="p-4 text-gray-500">"タイムラインを読み込み中..."</div> }
            }>
                {move || {
                    timeline
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

use crate::server::auth::get_current_user;
use crate::server::timeline::get_home_timeline;
