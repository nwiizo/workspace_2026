use leptos::prelude::*;
use leptos_router::hooks::use_params_map;

use crate::components::post_list::PostList;
use crate::components::user_avatar::UserAvatar;
use crate::server::social::FollowUser;

#[component]
pub fn ProfilePage() -> impl IntoView {
    let params = use_params_map();
    let username = move || params.get().get("username").unwrap_or_default();

    let profile = Resource::new(username, |u| async move { get_user_profile(u).await });

    let user_posts = Resource::new(username, |u| async move {
        get_user_timeline(u, None, None).await
    });

    let follow_action = ServerAction::<FollowUser>::new();

    view! {
        <div class="max-w-[600px] mx-auto border-x border-gray-800 min-h-screen">
            <Suspense fallback=move || {
                view! { <div class="p-4 text-gray-500">"読み込み中..."</div> }
            }>
                {move || {
                    profile
                        .get()
                        .map(|result| {
                            match result {
                                Ok(user) => {
                                    let uname = user.username.clone();
                                    let display_name = user.display_name.clone();
                                    let at_name = format!("@{}", user.username);
                                    let bio_text = user.bio.clone();

                                    view! {
                                        <div>
                                            <div class="h-32 bg-gray-800">
                                                {user
                                                    .header_url
                                                    .clone()
                                                    .map(|url| {
                                                        view! {
                                                            <img
                                                                src=url
                                                                class="w-full h-full object-cover"
                                                                alt="header"
                                                            />
                                                        }
                                                    })}
                                            </div>

                                            <div class="px-4 pb-4">
                                                <div class="flex justify-between items-end -mt-10">
                                                    <UserAvatar
                                                        url=user.avatar_url.clone()
                                                        size=80
                                                    />
                                                    <ActionForm action=follow_action>
                                                        <input
                                                            type="hidden"
                                                            name="target_username"
                                                            value=uname
                                                        />
                                                        <button
                                                            type="submit"
                                                            class=if user.is_following {
                                                                "px-4 py-1.5 border border-gray-600 rounded-full font-bold hover:border-red-500 hover:text-red-500"
                                                            } else {
                                                                "px-4 py-1.5 bg-white text-black rounded-full font-bold hover:bg-gray-200"
                                                            }
                                                        >
                                                            {if user.is_following {
                                                                "フォロー中"
                                                            } else {
                                                                "フォロー"
                                                            }}
                                                        </button>
                                                    </ActionForm>
                                                </div>

                                                <h2 class="text-xl font-bold mt-2">
                                                    {display_name}
                                                </h2>
                                                <p class="text-gray-500">{at_name}</p>

                                                {bio_text
                                                    .map(|bio| {
                                                        view! { <p class="mt-2">{bio}</p> }
                                                    })}

                                                <div class="flex gap-4 mt-3 text-sm">
                                                    <span>
                                                        {format!(
                                                            "{} フォロー中",
                                                            user.following_count,
                                                        )}
                                                    </span>
                                                    <span>
                                                        {format!(
                                                            "{} フォロワー",
                                                            user.followers_count,
                                                        )}
                                                    </span>
                                                </div>
                                            </div>
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

            <Suspense fallback=move || {
                view! { <div class="p-4 text-gray-500">"投稿を読み込み中..."</div> }
            }>
                {move || {
                    user_posts
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

use crate::server::social::get_user_profile;
use crate::server::timeline::get_user_timeline;
