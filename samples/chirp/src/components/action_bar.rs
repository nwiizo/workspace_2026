use leptos::prelude::*;

#[allow(unused_variables)]
#[component]
pub fn ActionBar(
    post_id: String,
    reply_count: i32,
    like_count: i32,
    rechirp_count: i32,
    liked: bool,
    rechirped: bool,
) -> impl IntoView {
    let (is_liked, set_liked) = signal(liked);
    let (local_like_count, set_like_count) = signal(like_count);
    let (is_rechirped, set_rechirped) = signal(rechirped);
    let (local_rechirp_count, set_rechirp_count) = signal(rechirp_count);

    let on_like = move |_| {
        let was_liked = is_liked.get_untracked();
        set_liked.set(!was_liked);
        set_like_count.update(|c| {
            if was_liked {
                *c -= 1;
            } else {
                *c += 1;
            }
        });
    };

    let on_rechirp = move |_| {
        let was_rechirped = is_rechirped.get_untracked();
        set_rechirped.set(!was_rechirped);
        set_rechirp_count.update(|c| {
            if was_rechirped {
                *c -= 1;
            } else {
                *c += 1;
            }
        });
    };

    view! {
        <div class="flex items-center gap-6 mt-2 text-gray-500 text-sm">
            // Reply
            <button class="flex items-center gap-1 hover:text-blue-400 transition-colors">
                <span>"💬"</span>
                <span>{reply_count}</span>
            </button>

            // Rechirp
            <button
                class=move || {
                    if is_rechirped.get() {
                        "flex items-center gap-1 text-green-500 transition-colors"
                    } else {
                        "flex items-center gap-1 hover:text-green-500 transition-colors"
                    }
                }
                on:click=on_rechirp
            >
                <span>"🔁"</span>
                <span>{move || local_rechirp_count.get()}</span>
            </button>

            // Like
            <button
                class=move || {
                    if is_liked.get() {
                        "flex items-center gap-1 text-pink-600 transition-colors"
                    } else {
                        "flex items-center gap-1 hover:text-pink-600 transition-colors"
                    }
                }
                on:click=on_like
            >
                <span>{move || if is_liked.get() { "❤️" } else { "🤍" }}</span>
                <span>{move || local_like_count.get()}</span>
            </button>
        </div>
    }
}
