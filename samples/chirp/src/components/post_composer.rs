use leptos::prelude::*;

use crate::components::user_avatar::UserAvatar;
use crate::server::posts::CreatePost;

#[component]
pub fn PostComposer(
    #[prop(optional_no_strip)] reply_to_id: Option<String>,
    #[prop(optional_no_strip)] avatar_url: Option<String>,
) -> impl IntoView {
    let create_post = ServerAction::<CreatePost>::new();
    let (content, set_content) = signal(String::new());
    let char_count = move || content.get().len();

    view! {
        <div class="p-4 border-b border-gray-800">
            <ActionForm action=create_post>
                <div class="flex gap-3">
                    <UserAvatar url=avatar_url size=40 />
                    <div class="flex-1">
                        <textarea
                            name="content"
                            placeholder="いまどうしてる？"
                            class="w-full bg-transparent text-xl resize-none outline-none placeholder-gray-600 min-h-[60px]"
                            maxlength="280"
                            prop:value=move || content.get()
                            on:input=move |ev| {
                                set_content
                                    .set(leptos::prelude::event_target_value(&ev));
                            }
                        />
                        {reply_to_id
                            .map(|id| {
                                view! {
                                    <input type="hidden" name="reply_to_id" value=id />
                                }
                            })}

                        <div class="flex items-center justify-between mt-2 pt-2 border-t border-gray-800">
                            <span
                                class=move || {
                                    if char_count() > 260 {
                                        "text-sm text-red-500"
                                    } else {
                                        "text-sm text-gray-500"
                                    }
                                }
                            >
                                {move || format!("{}/280", char_count())}
                            </span>
                            <button
                                type="submit"
                                class="px-5 py-1.5 bg-sky-500 hover:bg-sky-600 text-white font-bold rounded-full transition-colors disabled:opacity-50"
                                disabled=move || {
                                    content.get().trim().is_empty() || char_count() > 280
                                }
                            >
                                "Chirp"
                            </button>
                        </div>
                    </div>
                </div>
            </ActionForm>
        </div>
    }
}
