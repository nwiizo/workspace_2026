use leptos::prelude::*;

use crate::components::post_list::PostList;

#[component]
pub fn ExplorePage() -> impl IntoView {
    let timeline = Resource::new(|| (), |_| async { get_public_timeline(None, None).await });

    view! {
        <div class="max-w-[600px] mx-auto border-x border-gray-800 min-h-screen">
            <header class="sticky top-0 z-10 bg-black/80 backdrop-blur-md border-b border-gray-800">
                <h1 class="text-xl font-bold p-4">"探索"</h1>
            </header>

            <Suspense fallback=move || {
                view! { <div class="p-4 text-gray-500">"読み込み中..."</div> }
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

use crate::server::timeline::get_public_timeline;
