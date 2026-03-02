use leptos::prelude::*;
use leptos_router::hooks::use_params_map;
use uuid::Uuid;

use crate::components::ip_table::IpTable;
use crate::components::utilization_bar::UtilizationBar;
use crate::server::ip_addresses::{create_ip_address, list_ip_addresses};
use crate::server::subnets::{delete_subnet, get_subnet};

#[component]
pub fn SubnetDetailPage() -> impl IntoView {
    let params = use_params_map();
    let id = move || {
        params
            .read()
            .get("id")
            .and_then(|id| Uuid::parse_str(&id).ok())
    };

    let subnet_data = Resource::new(id, |id| async move {
        match id {
            Some(id) => get_subnet(id).await,
            None => Err(ServerFnError::new("Invalid subnet ID")),
        }
    });

    let ip_list = Resource::new(id, |id| async move {
        match id {
            Some(id) => list_ip_addresses(Some(id), None, None, None).await,
            None => Ok(Vec::new()),
        }
    });

    let (show_ip_form, set_show_ip_form) = signal(false);

    let add_ip_action = Action::new(move |input: &(Uuid, String, String, String)| {
        let (subnet_id, address, status, hostname) = input.clone();
        let hostname = if hostname.is_empty() {
            None
        } else {
            Some(hostname)
        };
        async move {
            let result =
                create_ip_address(address, subnet_id, status, hostname, None, None, None).await;
            if result.is_ok() {
                set_show_ip_form.set(false);
            }
            result
        }
    });

    let delete_action = Action::new(move |id: &Uuid| {
        let id = *id;
        async move { delete_subnet(id).await }
    });

    view! {
        <div>
            <Suspense fallback=|| view! { <div class="text-slate-400">"Loading..."</div> }>
                {move || subnet_data.get().map(|result| match result {
                    Ok(data) => {
                        let subnet = data.subnet.clone();
                        let subnet_id = subnet.id;
                        let name = subnet.name.clone();
                        let cidr = subnet.cidr.clone();
                        let desc = subnet.description.clone();

                        view! {
                            <div class="mb-6">
                                <div class="flex justify-between items-start">
                                    <div>
                                        <a href="/subnets" class="text-blue-400 hover:text-blue-300 text-sm mb-2 inline-block">
                                            "\u{2190} Back to Subnets"
                                        </a>
                                        <h1 class="text-2xl font-bold text-white">{name}</h1>
                                        <p class="text-blue-400 font-mono mt-1">{cidr}</p>
                                    </div>
                                    <button
                                        on:click=move |_| { delete_action.dispatch(subnet_id); }
                                        class="text-red-400 hover:text-red-300 text-sm px-3 py-1 border border-red-400/30 rounded-lg">
                                        "Delete"
                                    </button>
                                </div>

                                {desc.map(|d| view! {
                                    <p class="text-slate-400 mt-2">{d}</p>
                                })}

                                <div class="mt-4 max-w-md">
                                    <UtilizationBar used=subnet.used_count total=subnet.total_addresses />
                                </div>
                            </div>

                            {(!data.children.is_empty()).then(|| view! {
                                <div class="mb-6">
                                    <h2 class="text-lg font-semibold text-white mb-3">"Child Subnets"</h2>
                                    <div class="space-y-2">
                                        {data.children.into_iter().map(|child| {
                                            let href = format!("/subnets/{}", child.id);
                                            let child_name = child.name.clone();
                                            let child_cidr = child.cidr.clone();
                                            view! {
                                                <a href=href class="block bg-slate-800 border border-slate-700 rounded-lg p-3 hover:border-blue-500/50">
                                                    <span class="text-white font-medium">{child_name}</span>
                                                    " "
                                                    <span class="text-blue-400 font-mono text-sm">{child_cidr}</span>
                                                </a>
                                            }
                                        }).collect::<Vec<_>>()}
                                    </div>
                                </div>
                            })}

                            <div class="bg-slate-800 border border-slate-700 rounded-xl p-5">
                                <div class="flex justify-between items-center mb-4">
                                    <h2 class="text-lg font-semibold text-white">"IP Addresses"</h2>
                                    <button
                                        on:click=move |_| set_show_ip_form.update(|v| *v = !*v)
                                        class="bg-blue-500 hover:bg-blue-600 text-white px-3 py-1.5 rounded-lg text-sm transition-colors">
                                        "Add IP"
                                    </button>
                                </div>

                                {move || show_ip_form.get().then(|| {
                                    let sid = subnet_id;
                                    view! {
                                        <div class="mb-4 p-4 bg-slate-900 rounded-lg">
                                            <IpForm subnet_id=sid on_submit=move |address, status, hostname| {
                                                add_ip_action.dispatch((sid, address, status, hostname));
                                            } />
                                        </div>
                                    }
                                })}

                                <Suspense fallback=|| view! { <div class="text-slate-400">"Loading IPs..."</div> }>
                                    {move || ip_list.get().map(|result| match result {
                                        Ok(ips) => {
                                            if ips.is_empty() {
                                                view! { <p class="text-slate-500 text-sm">"No IP addresses in this subnet"</p> }.into_any()
                                            } else {
                                                view! { <IpTable ips=ips /> }.into_any()
                                            }
                                        },
                                        Err(e) => view! { <p class="text-red-400">{e.to_string()}</p> }.into_any(),
                                    })}
                                </Suspense>
                            </div>
                        }.into_any()
                    },
                    Err(e) => view! {
                        <div>
                            <a href="/subnets" class="text-blue-400 hover:text-blue-300 text-sm">"\u{2190} Back to Subnets"</a>
                            <p class="text-red-400 mt-4">{e.to_string()}</p>
                        </div>
                    }.into_any(),
                })}
            </Suspense>
        </div>
    }
}

#[component]
fn IpForm<F>(subnet_id: Uuid, on_submit: F) -> impl IntoView
where
    F: Fn(String, String, String) + 'static + Clone,
{
    let _ = subnet_id;
    let (address, set_address) = signal(String::new());
    let (status, set_status) = signal("available".to_string());
    let (hostname, set_hostname) = signal(String::new());

    view! {
        <form on:submit=move |ev: leptos::ev::SubmitEvent| {
            ev.prevent_default();
            let on_submit = on_submit.clone();
            on_submit(address.get(), status.get(), hostname.get());
        } class="space-y-3">
            <div class="grid grid-cols-1 md:grid-cols-3 gap-3">
                <div>
                    <label class="block text-xs text-slate-400 mb-1">"IP Address"</label>
                    <input type="text"
                        placeholder="192.168.1.10"
                        prop:value=move || address.get()
                        on:input=move |ev| set_address.set(event_target_value(&ev))
                        class="w-full bg-slate-950 border border-slate-600 rounded px-3 py-1.5 text-white text-sm focus:border-blue-500 focus:outline-none" />
                </div>
                <div>
                    <label class="block text-xs text-slate-400 mb-1">"Status"</label>
                    <select
                        prop:value=move || status.get()
                        on:change=move |ev: leptos::ev::Event| set_status.set(event_target_value(&ev))
                        class="w-full bg-slate-950 border border-slate-600 rounded px-3 py-1.5 text-white text-sm focus:border-blue-500 focus:outline-none">
                        <option value="available">"Available"</option>
                        <option value="assigned">"Assigned"</option>
                        <option value="reserved">"Reserved"</option>
                        <option value="deprecated">"Deprecated"</option>
                    </select>
                </div>
                <div>
                    <label class="block text-xs text-slate-400 mb-1">"Hostname"</label>
                    <input type="text"
                        placeholder="web-server-01"
                        prop:value=move || hostname.get()
                        on:input=move |ev| set_hostname.set(event_target_value(&ev))
                        class="w-full bg-slate-950 border border-slate-600 rounded px-3 py-1.5 text-white text-sm focus:border-blue-500 focus:outline-none" />
                </div>
            </div>
            <button type="submit"
                class="bg-blue-500 hover:bg-blue-600 text-white px-3 py-1.5 rounded text-sm transition-colors">
                "Add IP"
            </button>
        </form>
    }
}
