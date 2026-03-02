use leptos::prelude::*;

use crate::components::subnet_card::SubnetCard;
use crate::server::subnets::{create_subnet, list_subnets};

#[component]
pub fn SubnetListPage() -> impl IntoView {
    let subnets = Resource::new(|| (), |_| list_subnets());
    let (show_form, set_show_form) = signal(false);

    let create_action = Action::new(move |input: &(String, String, String)| {
        let (cidr, name, desc) = input.clone();
        let desc = if desc.is_empty() { None } else { Some(desc) };
        async move {
            let result = create_subnet(cidr, name, desc, None).await;
            if result.is_ok() {
                set_show_form.set(false);
            }
            result
        }
    });

    view! {
        <div>
            <div class="flex justify-between items-center mb-6">
                <h1 class="text-2xl font-bold text-white">"Subnets"</h1>
                <button
                    on:click=move |_| set_show_form.update(|v| *v = !*v)
                    class="bg-blue-500 hover:bg-blue-600 text-white px-4 py-2 rounded-lg text-sm font-medium transition-colors">
                    "Add Subnet"
                </button>
            </div>

            {move || show_form.get().then(|| view! {
                <div class="bg-slate-800 border border-slate-700 rounded-xl p-5 mb-6">
                    <h2 class="text-lg font-semibold text-white mb-4">"New Subnet"</h2>
                    <SubnetForm on_submit=move |cidr, name, desc| {
                        create_action.dispatch((cidr, name, desc));
                    } />
                </div>
            })}

            <Suspense fallback=|| view! { <div class="text-slate-400">"Loading subnets..."</div> }>
                {move || subnets.get().map(|result| match result {
                    Ok(list) => {
                        if list.is_empty() {
                            view! {
                                <div class="text-center py-12 text-slate-500">
                                    <p class="text-lg">"No subnets yet"</p>
                                    <p class="text-sm mt-1">"Add your first subnet to get started"</p>
                                </div>
                            }.into_any()
                        } else {
                            view! {
                                <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                                    {list.into_iter().map(|subnet| view! {
                                        <SubnetCard subnet=subnet />
                                    }).collect::<Vec<_>>()}
                                </div>
                            }.into_any()
                        }
                    },
                    Err(e) => view! { <p class="text-red-400">{e.to_string()}</p> }.into_any(),
                })}
            </Suspense>
        </div>
    }
}

#[component]
fn SubnetForm<F>(on_submit: F) -> impl IntoView
where
    F: Fn(String, String, String) + 'static + Clone,
{
    let (cidr, set_cidr) = signal(String::new());
    let (name, set_name) = signal(String::new());
    let (desc, set_desc) = signal(String::new());

    view! {
        <form on:submit=move |ev: leptos::ev::SubmitEvent| {
            ev.prevent_default();
            let on_submit = on_submit.clone();
            on_submit(cidr.get(), name.get(), desc.get());
        } class="space-y-4">
            <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                <div>
                    <label class="block text-sm text-slate-400 mb-1">"CIDR"</label>
                    <input type="text"
                        placeholder="192.168.1.0/24"
                        prop:value=move || cidr.get()
                        on:input=move |ev| set_cidr.set(event_target_value(&ev))
                        class="w-full bg-slate-900 border border-slate-600 rounded-lg px-3 py-2 text-white text-sm focus:border-blue-500 focus:outline-none" />
                </div>
                <div>
                    <label class="block text-sm text-slate-400 mb-1">"Name"</label>
                    <input type="text"
                        placeholder="Production Network"
                        prop:value=move || name.get()
                        on:input=move |ev| set_name.set(event_target_value(&ev))
                        class="w-full bg-slate-900 border border-slate-600 rounded-lg px-3 py-2 text-white text-sm focus:border-blue-500 focus:outline-none" />
                </div>
            </div>
            <div>
                <label class="block text-sm text-slate-400 mb-1">"Description"</label>
                <input type="text"
                    placeholder="Optional description"
                    prop:value=move || desc.get()
                    on:input=move |ev| set_desc.set(event_target_value(&ev))
                    class="w-full bg-slate-900 border border-slate-600 rounded-lg px-3 py-2 text-white text-sm focus:border-blue-500 focus:outline-none" />
            </div>
            <button type="submit"
                class="bg-blue-500 hover:bg-blue-600 text-white px-4 py-2 rounded-lg text-sm font-medium transition-colors">
                "Create Subnet"
            </button>
        </form>
    }
}
