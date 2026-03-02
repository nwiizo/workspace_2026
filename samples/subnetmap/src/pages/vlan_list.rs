use leptos::prelude::*;

use crate::server::vlans::{create_vlan, list_vlans};

#[component]
pub fn VlanListPage() -> impl IntoView {
    let vlans = Resource::new(|| (), |_| list_vlans());
    let (show_form, set_show_form) = signal(false);

    let create_action = Action::new(move |input: &(String, String, String)| {
        let (vlan_id_str, name, desc) = input.clone();
        async move {
            let vlan_id: i32 = vlan_id_str
                .parse()
                .map_err(|_| ServerFnError::new("Invalid VLAN ID"))?;
            let desc = if desc.is_empty() { None } else { Some(desc) };
            let result = create_vlan(vlan_id, name, desc).await;
            if result.is_ok() {
                set_show_form.set(false);
            }
            result
        }
    });

    view! {
        <div>
            <div class="flex justify-between items-center mb-6">
                <h1 class="text-2xl font-bold text-white">"VLANs"</h1>
                <button
                    on:click=move |_| set_show_form.update(|v| *v = !*v)
                    class="bg-blue-500 hover:bg-blue-600 text-white px-4 py-2 rounded-lg text-sm font-medium transition-colors">
                    "Add VLAN"
                </button>
            </div>

            {move || show_form.get().then(|| view! {
                <div class="bg-slate-800 border border-slate-700 rounded-xl p-5 mb-6">
                    <h2 class="text-lg font-semibold text-white mb-4">"New VLAN"</h2>
                    <VlanForm on_submit=move |vid, name, desc| {
                        create_action.dispatch((vid, name, desc));
                    } />
                </div>
            })}

            <div class="bg-slate-800 border border-slate-700 rounded-xl">
                <Suspense fallback=|| view! { <div class="p-5 text-slate-400">"Loading..."</div> }>
                    {move || vlans.get().map(|result| match result {
                        Ok(list) => {
                            if list.is_empty() {
                                view! {
                                    <div class="text-center py-12 text-slate-500">
                                        <p>"No VLANs configured"</p>
                                    </div>
                                }.into_any()
                            } else {
                                view! {
                                    <table class="w-full text-sm">
                                        <thead>
                                            <tr class="border-b border-slate-700 text-slate-400 text-left">
                                                <th class="p-4 font-medium">"VLAN ID"</th>
                                                <th class="p-4 font-medium">"Name"</th>
                                                <th class="p-4 font-medium">"Description"</th>
                                                <th class="p-4 font-medium">"Created"</th>
                                            </tr>
                                        </thead>
                                        <tbody>
                                            {list.into_iter().map(|vlan| {
                                                let href = format!("/vlans/{}", vlan.id);
                                                let vid = vlan.vlan_id;
                                                let name = vlan.name.clone();
                                                let desc = vlan.description.clone().unwrap_or_else(|| "\u{2014}".to_string());
                                                let created = vlan.created_at.format("%Y-%m-%d").to_string();
                                                view! {
                                                    <tr class="border-b border-slate-800 hover:bg-slate-800/50">
                                                        <td class="p-4">
                                                            <a href=href class="text-blue-400 hover:text-blue-300 font-mono">
                                                                {vid}
                                                            </a>
                                                        </td>
                                                        <td class="p-4 text-white">{name}</td>
                                                        <td class="p-4 text-slate-400">{desc}</td>
                                                        <td class="p-4 text-slate-500 text-xs">{created}</td>
                                                    </tr>
                                                }
                                            }).collect::<Vec<_>>()}
                                        </tbody>
                                    </table>
                                }.into_any()
                            }
                        },
                        Err(e) => view! { <p class="p-5 text-red-400">{e.to_string()}</p> }.into_any(),
                    })}
                </Suspense>
            </div>
        </div>
    }
}

#[component]
fn VlanForm<F>(on_submit: F) -> impl IntoView
where
    F: Fn(String, String, String) + 'static + Clone,
{
    let (vlan_id, set_vlan_id) = signal(String::new());
    let (name, set_name) = signal(String::new());
    let (desc, set_desc) = signal(String::new());

    view! {
        <form on:submit=move |ev: leptos::ev::SubmitEvent| {
            ev.prevent_default();
            let on_submit = on_submit.clone();
            on_submit(vlan_id.get(), name.get(), desc.get());
        } class="space-y-4">
            <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
                <div>
                    <label class="block text-sm text-slate-400 mb-1">"VLAN ID"</label>
                    <input type="number" min="1" max="4094"
                        placeholder="100"
                        prop:value=move || vlan_id.get()
                        on:input=move |ev| set_vlan_id.set(event_target_value(&ev))
                        class="w-full bg-slate-900 border border-slate-600 rounded-lg px-3 py-2 text-white text-sm focus:border-blue-500 focus:outline-none" />
                </div>
                <div>
                    <label class="block text-sm text-slate-400 mb-1">"Name"</label>
                    <input type="text"
                        placeholder="Management"
                        prop:value=move || name.get()
                        on:input=move |ev| set_name.set(event_target_value(&ev))
                        class="w-full bg-slate-900 border border-slate-600 rounded-lg px-3 py-2 text-white text-sm focus:border-blue-500 focus:outline-none" />
                </div>
                <div>
                    <label class="block text-sm text-slate-400 mb-1">"Description"</label>
                    <input type="text"
                        placeholder="Optional"
                        prop:value=move || desc.get()
                        on:input=move |ev| set_desc.set(event_target_value(&ev))
                        class="w-full bg-slate-900 border border-slate-600 rounded-lg px-3 py-2 text-white text-sm focus:border-blue-500 focus:outline-none" />
                </div>
            </div>
            <button type="submit"
                class="bg-blue-500 hover:bg-blue-600 text-white px-4 py-2 rounded-lg text-sm font-medium transition-colors">
                "Create VLAN"
            </button>
        </form>
    }
}
