use leptos::prelude::*;

#[component]
pub fn UserAvatar(
    #[prop(optional_no_strip)] url: Option<String>,
    #[prop(default = 40)] size: u32,
) -> impl IntoView {
    let size_class = match size {
        0..=32 => "w-8 h-8",
        33..=48 => "w-10 h-10",
        49..=64 => "w-12 h-12",
        _ => "w-16 h-16",
    };

    let class_str = format!("{size_class} rounded-full bg-gray-600 overflow-hidden flex-shrink-0");

    view! {
        <div class=class_str>
            {url
                .map(|u| {
                    view! {
                        <img src=u class="w-full h-full object-cover" alt="avatar" />
                    }
                })}
        </div>
    }
}
