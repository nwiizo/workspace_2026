use leptos::prelude::*;

use crate::models::ip_address::IpStatus;

#[component]
pub fn StatusBadge(#[prop(into)] status: String) -> impl IntoView {
    let parsed = IpStatus::parse(&status);
    let class = format!(
        "inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium {}",
        parsed.color_class()
    );

    view! {
        <span class=class>{status}</span>
    }
}
