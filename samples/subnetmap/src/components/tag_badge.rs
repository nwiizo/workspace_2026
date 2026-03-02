use leptos::prelude::*;

use crate::models::tag::Tag;

#[component]
pub fn TagBadge(tag: Tag) -> impl IntoView {
    let style = format!("background-color: {}20; color: {}", tag.color, tag.color);
    let name = tag.name.clone();

    view! {
        <span class="inline-flex items-center px-2 py-0.5 rounded text-xs font-medium"
              style=style>
            {name}
        </span>
    }
}
