use leptos::prelude::*;

use crate::server::auth::Logout;

#[component]
pub fn SettingsPage() -> impl IntoView {
    let current_user = Resource::new(|| (), |_| async { get_current_user().await });
    let logout_action = ServerAction::<Logout>::new();

    view! {
        <div class="max-w-[600px] mx-auto border-x border-gray-800 min-h-screen">
            <header class="sticky top-0 z-10 bg-black/80 backdrop-blur-md border-b border-gray-800">
                <h1 class="text-xl font-bold p-4">"設定"</h1>
            </header>

            <Suspense fallback=move || {
                view! { <div class="p-4 text-gray-500">"読み込み中..."</div> }
            }>
                {move || {
                    current_user
                        .get()
                        .map(|result| {
                            match result {
                                Ok(Some(user)) => {
                                    let display = user.display_name.clone();
                                    let uname = format!("@{}", user.username);
                                    view! {
                                        <div class="p-4 space-y-6">
                                            <div>
                                                <h2 class="text-lg font-bold mb-2">
                                                    "アカウント情報"
                                                </h2>
                                                <p class="text-gray-400">
                                                    {format!("表示名: {display}")}
                                                </p>
                                                <p class="text-gray-400">
                                                    {format!("ユーザー名: {uname}")}
                                                </p>
                                            </div>

                                            <div class="pt-4 border-t border-gray-800">
                                                <ActionForm action=logout_action>
                                                    <button
                                                        type="submit"
                                                        class="px-6 py-2 bg-red-600 hover:bg-red-700 text-white rounded-full font-bold transition-colors"
                                                    >
                                                        "ログアウト"
                                                    </button>
                                                </ActionForm>
                                            </div>
                                        </div>
                                    }
                                        .into_any()
                                }
                                Ok(None) => {
                                    view! {
                                        <div class="p-4 text-gray-500">
                                            "ログインしてください"
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

use crate::server::auth::get_current_user;
