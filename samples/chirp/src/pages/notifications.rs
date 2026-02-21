use leptos::prelude::*;
use leptos_router::components::A;

use crate::components::user_avatar::UserAvatar;
use crate::models::notification::NotificationEvent;

#[component]
pub fn NotificationsPage() -> impl IntoView {
    let notifications = Resource::new(|| (), |_| async { get_notifications(None, None).await });

    view! {
        <div class="max-w-[600px] mx-auto border-x border-gray-800 min-h-screen">
            <header class="sticky top-0 z-10 bg-black/80 backdrop-blur-md border-b border-gray-800">
                <h1 class="text-xl font-bold p-4">"通知"</h1>
            </header>

            <Suspense fallback=move || {
                view! { <div class="p-4 text-gray-500">"読み込み中..."</div> }
            }>
                {move || {
                    notifications
                        .get()
                        .map(|result| {
                            match result {
                                Ok(notifs) => {
                                    if notifs.is_empty() {
                                        view! {
                                            <div class="p-8 text-center text-gray-500">
                                                "通知はまだありません"
                                            </div>
                                        }
                                            .into_any()
                                    } else {
                                        view! {
                                            <div>
                                                {notifs
                                                    .into_iter()
                                                    .map(|n| {
                                                        let msg = match &n.event_type {
                                                            NotificationEvent::Like => "がいいねしました",
                                                            NotificationEvent::Rechirp => "がリチャープしました",
                                                            NotificationEvent::Follow => "がフォローしました",
                                                            NotificationEvent::Reply => "が返信しました",
                                                            NotificationEvent::Mention => "がメンションしました",
                                                        };
                                                        let href = match &n.event_type {
                                                            NotificationEvent::Follow => {
                                                                format!("/user/{}", n.actor.username)
                                                            }
                                                            _ => {
                                                                n
                                                                    .post_id
                                                                    .map(|id| format!("/post/{id}"))
                                                                    .unwrap_or_else(|| "/".to_string())
                                                            }
                                                        };
                                                        let bg = if n.is_read {
                                                            ""
                                                        } else {
                                                            "bg-sky-500/5"
                                                        };
                                                        let actor_name = n.actor.display_name.clone();
                                                        let preview = n
                                                            .post_content
                                                            .as_ref()
                                                            .map(|c| c.chars().take(100).collect::<String>());
                                                        view! {
                                                            <A
                                                                href=href
                                                                attr:class=format!(
                                                                    "block p-4 border-b border-gray-800 hover:bg-gray-900/50 {bg}",
                                                                )
                                                            >
                                                                <div class="flex gap-3">
                                                                    <UserAvatar
                                                                        url=n.actor.avatar_url.clone()
                                                                        size=32
                                                                    />
                                                                    <div>
                                                                        <p>
                                                                            {format!(
                                                                                "{}{}", actor_name, msg,
                                                                            )}
                                                                        </p>
                                                                        {preview
                                                                            .map(|p| {
                                                                                view! {
                                                                                    <p class="text-gray-500 text-sm mt-1">
                                                                                        {p}
                                                                                    </p>
                                                                                }
                                                                            })}
                                                                    </div>
                                                                </div>
                                                            </A>
                                                        }
                                                    })
                                                    .collect::<Vec<_>>()}
                                            </div>
                                        }
                                            .into_any()
                                    }
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

use crate::server::notifications::get_notifications;
