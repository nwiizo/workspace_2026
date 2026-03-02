use leptos::prelude::*;

use crate::components::user_avatar::UserAvatar;
use crate::server::posts::CreatePost;

/// PostComposer — 投稿フォーム（送信後の自動クリア付き）
///
/// このコンポーネントは Leptos の Effect パターンを示す:
///
/// - `ServerAction::value()` は `RwSignal<Option<Result<T, E>>>` を返す
/// - Effect でこの Signal を監視し、成功時にフォームをクリアする
///
/// React では `useEffect(() => { if (result) reset() }, [result])` と書く。
/// Leptos の Effect も同じ概念だが、依存関係の追跡が自動的かつ正確だ。
/// React の useEffect は依存配列の指定漏れでバグになりうるが、
/// Leptos の Effect は `.get()` を呼んだ Signal を自動追跡する。
#[component]
pub fn PostComposer(
    #[prop(optional_no_strip)] reply_to_id: Option<String>,
    #[prop(optional_no_strip)] avatar_url: Option<String>,
) -> impl IntoView {
    let create_post = ServerAction::<CreatePost>::new();
    let (content, set_content) = signal(String::new());
    let char_count = move || content.get().chars().count();

    // --- Effect: ServerAction の結果を監視する ---
    //
    // Effect は「リアクティブな値が変わったときに副作用を実行する」仕組み。
    // view! の中では Signal → DOM の自動更新が起きるが、
    // 「フォームをクリアする」のは DOM の更新ではなく副作用 (side effect) だ。
    //
    // JavaScript のフロントエンドでは、副作用の管理は常に課題だった:
    // - jQuery: コールバックの中で何でもできてしまう → スパゲッティコード
    // - React: useEffect の依存配列の指定漏れ → 無限ループやstaleクロージャ
    // - Leptos: .get() の呼び出しが自動的に依存を登録 → 漏れがない
    let action_value = create_post.value();
    Effect::new(move || {
        // action_value.get() を呼ぶことで、この Effect は
        // action_value の変更時に自動的に再実行される
        if let Some(Ok(_)) = action_value.get() {
            set_content.set(String::new());
        }
    });

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
                                    create_post.pending().get()
                                        || content.get().trim().is_empty()
                                        || char_count() > 280
                                }
                            >
                                {move || {
                                    if create_post.pending().get() {
                                        "送信中..."
                                    } else {
                                        "Chirp"
                                    }
                                }}
                            </button>
                        </div>
                    </div>
                </div>
            </ActionForm>
        </div>
    }
}
