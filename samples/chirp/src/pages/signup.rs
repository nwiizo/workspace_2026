use leptos::prelude::*;
use leptos_router::components::A;

use crate::server::auth::Signup;

#[component]
pub fn SignupPage() -> impl IntoView {
    let signup_action = ServerAction::<Signup>::new();
    let value = signup_action.value();

    view! {
        <div class="min-h-screen flex items-center justify-center">
            <div class="w-full max-w-md p-8">
                <h1 class="text-3xl font-bold mb-2 text-center">"アカウントを作成"</h1>
                <p class="text-chirp-secondary text-center mb-8">"Join Chirp today!"</p>

                <ActionForm action=signup_action>
                    <div class="space-y-4">
                        <div>
                            <label class="block text-sm font-medium mb-1" for="display_name">
                                "表示名"
                            </label>
                            <input
                                type="text"
                                name="display_name"
                                id="display_name"
                                required
                                class="w-full px-4 py-3 bg-transparent border border-chirp-border rounded-lg focus:outline-none focus:border-chirp-blue"
                                placeholder="表示名"
                            />
                        </div>
                        <div>
                            <label class="block text-sm font-medium mb-1" for="username">
                                "ユーザー名"
                            </label>
                            <input
                                type="text"
                                name="username"
                                id="username"
                                required
                                class="w-full px-4 py-3 bg-transparent border border-chirp-border rounded-lg focus:outline-none focus:border-chirp-blue"
                                placeholder="username"
                            />
                        </div>
                        <div>
                            <label class="block text-sm font-medium mb-1" for="email">
                                "メールアドレス"
                            </label>
                            <input
                                type="email"
                                name="email"
                                id="email"
                                required
                                class="w-full px-4 py-3 bg-transparent border border-chirp-border rounded-lg focus:outline-none focus:border-chirp-blue"
                                placeholder="email@example.com"
                            />
                        </div>
                        <div>
                            <label class="block text-sm font-medium mb-1" for="password">
                                "パスワード"
                            </label>
                            <input
                                type="password"
                                name="password"
                                id="password"
                                required
                                minlength="8"
                                class="w-full px-4 py-3 bg-transparent border border-chirp-border rounded-lg focus:outline-none focus:border-chirp-blue"
                                placeholder="8文字以上"
                            />
                        </div>

                        {move || {
                            value
                                .get()
                                .and_then(|r: Result<(), ServerFnError>| r.err())
                                .map(|e: ServerFnError| {
                                    view! {
                                        <p class="text-red-500 text-sm">{e.to_string()}</p>
                                    }
                                })
                        }}

                        <button
                            type="submit"
                            class="w-full py-3 bg-chirp-blue hover:bg-chirp-hover text-white font-bold rounded-full transition-colors"
                        >
                            "アカウント作成"
                        </button>
                    </div>
                </ActionForm>

                <p class="mt-6 text-center text-chirp-secondary">
                    "すでにアカウントをお持ちですか？ "
                    <A href="/login" attr:class="text-chirp-blue hover:underline">
                        "ログイン"
                    </A>
                </p>
            </div>
        </div>
    }
}
