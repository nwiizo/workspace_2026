use leptos::prelude::*;
use leptos_router::components::A;

use crate::server::auth::Login;

#[component]
pub fn LoginPage() -> impl IntoView {
    let login_action = ServerAction::<Login>::new();
    let value = login_action.value();

    view! {
        <div class="min-h-screen flex items-center justify-center">
            <div class="w-full max-w-md p-8">
                <h1 class="text-3xl font-bold mb-2 text-center">"Chirp にログイン"</h1>
                <p class="text-chirp-secondary text-center mb-8">"Welcome back!"</p>

                <ActionForm action=login_action>
                    <div class="space-y-4">
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
                            <label class="block text-sm font-medium mb-1" for="password">
                                "パスワード"
                            </label>
                            <input
                                type="password"
                                name="password"
                                id="password"
                                required
                                class="w-full px-4 py-3 bg-transparent border border-chirp-border rounded-lg focus:outline-none focus:border-chirp-blue"
                                placeholder="••••••••"
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
                            "ログイン"
                        </button>
                    </div>
                </ActionForm>

                <p class="mt-6 text-center text-chirp-secondary">
                    "アカウントをお持ちでないですか？ "
                    <A href="/signup" attr:class="text-chirp-blue hover:underline">
                        "新規登録"
                    </A>
                </p>
            </div>
        </div>
    }
}
