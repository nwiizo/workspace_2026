use leptos::prelude::*;

use crate::components::post_card::PostCard;
use crate::models::post::PostWithMeta;

#[component]
pub fn PostList(posts: Vec<PostWithMeta>) -> impl IntoView {
    if posts.is_empty() {
        return view! {
            <div class="p-8 text-center text-gray-500">
                "まだ投稿がありません"
            </div>
        }
        .into_any();
    }

    view! {
        <div>
            {posts
                .into_iter()
                .map(|post| view! { <PostCard post=post /> })
                .collect::<Vec<_>>()}
        </div>
    }
    .into_any()
}
