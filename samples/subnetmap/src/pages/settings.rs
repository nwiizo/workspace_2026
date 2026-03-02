use leptos::prelude::*;

use crate::server::tags::{create_tag, delete_tag, list_tags};

#[component]
pub fn SettingsPage() -> impl IntoView {
    let tags = Resource::new(|| (), |_| list_tags());
    let (show_form, set_show_form) = signal(false);

    let create_action = Action::new(move |input: &(String, String)| {
        let (name, color) = input.clone();
        async move {
            let result = create_tag(name, color).await;
            if result.is_ok() {
                set_show_form.set(false);
            }
            result
        }
    });

    let delete_action = Action::new(move |id: &uuid::Uuid| {
        let id = *id;
        async move { delete_tag(id).await }
    });

    view! {
        <div>
            <h1 class="text-2xl font-bold text-white mb-6">"Settings"</h1>

            <div class="bg-slate-800 border border-slate-700 rounded-xl p-5">
                <div class="flex justify-between items-center mb-4">
                    <h2 class="text-lg font-semibold text-white">"Tags"</h2>
                    <button
                        on:click=move |_| set_show_form.update(|v| *v = !*v)
                        class="bg-blue-500 hover:bg-blue-600 text-white px-3 py-1.5 rounded-lg text-sm transition-colors">
                        "Add Tag"
                    </button>
                </div>

                {move || show_form.get().then(|| view! {
                    <div class="mb-4 p-4 bg-slate-900 rounded-lg">
                        <TagForm on_submit=move |name, color| {
                            create_action.dispatch((name, color));
                        } />
                    </div>
                })}

                <Suspense fallback=|| view! { <div class="text-slate-400">"Loading..."</div> }>
                    {move || tags.get().map(|result| match result {
                        Ok(list) => {
                            if list.is_empty() {
                                view! { <p class="text-slate-500 text-sm">"No tags created"</p> }.into_any()
                            } else {
                                view! {
                                    <div class="space-y-2">
                                        {list.into_iter().map(|tag| {
                                            let tag_id = tag.id;
                                            let style = format!("background-color: {}20; border-color: {}40", tag.color, tag.color);
                                            let dot_style = format!("background-color: {}", tag.color);
                                            let name = tag.name.clone();
                                            let color_hex = tag.color.clone();
                                            view! {
                                                <div class="flex justify-between items-center p-3 rounded-lg border"
                                                     style=style>
                                                    <div class="flex items-center gap-3">
                                                        <div class="w-3 h-3 rounded-full" style=dot_style></div>
                                                        <span class="text-white text-sm">{name}</span>
                                                        <span class="text-slate-500 text-xs font-mono">{color_hex}</span>
                                                    </div>
                                                    <button
                                                        on:click=move |_| { delete_action.dispatch(tag_id); }
                                                        class="text-red-400 hover:text-red-300 text-xs">
                                                        "Delete"
                                                    </button>
                                                </div>
                                            }
                                        }).collect::<Vec<_>>()}
                                    </div>
                                }.into_any()
                            }
                        },
                        Err(e) => view! { <p class="text-red-400">{e.to_string()}</p> }.into_any(),
                    })}
                </Suspense>
            </div>
        </div>
    }
}

#[component]
fn TagForm<F>(on_submit: F) -> impl IntoView
where
    F: Fn(String, String) + 'static + Clone,
{
    let (name, set_name) = signal(String::new());
    let (color, set_color) = signal("#3b82f6".to_string());

    view! {
        <form on:submit=move |ev: leptos::ev::SubmitEvent| {
            ev.prevent_default();
            let on_submit = on_submit.clone();
            on_submit(name.get(), color.get());
        } class="flex gap-3 items-end">
            <div class="flex-1">
                <label class="block text-xs text-slate-400 mb-1">"Name"</label>
                <input type="text"
                    placeholder="production"
                    prop:value=move || name.get()
                    on:input=move |ev| set_name.set(event_target_value(&ev))
                    class="w-full bg-slate-950 border border-slate-600 rounded px-3 py-1.5 text-white text-sm" />
            </div>
            <div>
                <label class="block text-xs text-slate-400 mb-1">"Color"</label>
                <input type="color"
                    prop:value=move || color.get()
                    on:input=move |ev| set_color.set(event_target_value(&ev))
                    class="h-8 w-16 rounded cursor-pointer bg-transparent" />
            </div>
            <button type="submit"
                class="bg-blue-500 hover:bg-blue-600 text-white px-3 py-1.5 rounded text-sm">
                "Create"
            </button>
        </form>
    }
}
