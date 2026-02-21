use leptos::prelude::*;

#[component]
pub fn RightSidebar() -> impl IntoView {
    view! {
        <aside class="w-80 flex-shrink-0 hidden xl:block p-4 sticky top-0 h-screen">
            <div class="space-y-4">
                // Search box
                <form action="/search" method="get">
                    <input
                        type="text"
                        name="q"
                        placeholder="検索..."
                        class="w-full px-4 py-2 bg-gray-900 border border-gray-700 rounded-full focus:outline-none focus:border-sky-500 text-sm"
                    />
                </form>

                // Trending / info section
                <div class="bg-gray-900 rounded-xl p-4">
                    <h2 class="text-xl font-bold mb-4">"Chirp について"</h2>
                    <p class="text-gray-400 text-sm">
                        "Rust + Leptos で構築された Twitter 風 SNS のサンプルアプリケーションです。"
                    </p>
                </div>

                <div class="bg-gray-900 rounded-xl p-4">
                    <h2 class="text-lg font-bold mb-3">"技術スタック"</h2>
                    <ul class="text-gray-400 text-sm space-y-2">
                        <li>"Leptos 0.8 (SSR + Hydration)"</li>
                        <li>"Axum 0.8"</li>
                        <li>"SQLx + PostgreSQL 17"</li>
                        <li>"Tailwind CSS v4"</li>
                        <li>"tower-sessions + argon2"</li>
                    </ul>
                </div>
            </div>
        </aside>
    }
}
