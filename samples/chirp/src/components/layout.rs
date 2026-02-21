use leptos::prelude::*;

use crate::components::nav_sidebar::NavSidebar;
use crate::components::right_sidebar::RightSidebar;

#[component]
pub fn Layout(children: Children) -> impl IntoView {
    view! {
        <div class="flex justify-center min-h-screen">
            <NavSidebar />
            <main class="flex-1 max-w-[600px]">{children()}</main>
            <RightSidebar />
        </div>
    }
}
