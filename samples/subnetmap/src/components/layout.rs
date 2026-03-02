use leptos::prelude::*;

use super::nav_sidebar::NavSidebar;

#[component]
pub fn Layout(children: Children) -> impl IntoView {
    view! {
        <div class="flex min-h-screen">
            <NavSidebar />
            <main class="flex-1 ml-64 p-6">
                <div class="max-w-7xl mx-auto">
                    {children()}
                </div>
            </main>
        </div>
    }
}
