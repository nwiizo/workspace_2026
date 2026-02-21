use leptos::prelude::*;
use leptos_router::components::A;

use crate::components::action_bar::ActionBar;
use crate::components::user_avatar::UserAvatar;
use crate::models::post::PostWithMeta;

#[component]
pub fn PostCard(post: PostWithMeta) -> impl IntoView {
    let post_url = format!("/post/{}", post.id);
    let user_url = format!("/user/{}", post.author.username);
    let time_ago = format_time_ago(post.created_at);
    let at_username = format!("@{}", post.author.username);
    let display_name = post.author.display_name.clone();
    let content = post.content.clone();

    view! {
        <article class="p-4 border-b border-gray-800 hover:bg-gray-900/50 transition-colors">
            <div class="flex gap-3">
                <A href=user_url.clone()>
                    <UserAvatar url=post.author.avatar_url.clone() size=40 />
                </A>
                <div class="flex-1 min-w-0">
                    <div class="flex items-center gap-1 text-sm">
                        <A href=user_url.clone() attr:class="font-bold hover:underline truncate">
                            {display_name}
                        </A>
                        <A href=user_url attr:class="text-gray-500 truncate">
                            {at_username}
                        </A>
                        <span class="text-gray-500">"·"</span>
                        <A href=post_url attr:class="text-gray-500 hover:underline">
                            {time_ago}
                        </A>
                    </div>
                    <div class="mt-1 whitespace-pre-wrap break-words">{content}</div>
                    <ActionBar
                        post_id=post.id.to_string()
                        reply_count=post.reply_count
                        like_count=post.like_count
                        rechirp_count=post.rechirp_count
                        liked=post.liked_by_me
                        rechirped=post.rechirped_by_me
                    />
                </div>
            </div>
        </article>
    }
}

fn format_time_ago(dt: chrono::DateTime<chrono::Utc>) -> String {
    let now = chrono::Utc::now();
    let diff = now - dt;

    if diff.num_seconds() < 60 {
        format!("{}s", diff.num_seconds().max(0))
    } else if diff.num_minutes() < 60 {
        format!("{}m", diff.num_minutes())
    } else if diff.num_hours() < 24 {
        format!("{}h", diff.num_hours())
    } else if diff.num_days() < 7 {
        format!("{}d", diff.num_days())
    } else {
        dt.format("%m/%d").to_string()
    }
}
