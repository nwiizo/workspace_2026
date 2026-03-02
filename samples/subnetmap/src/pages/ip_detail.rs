use leptos::prelude::*;
use leptos_router::hooks::use_params_map;
use uuid::Uuid;

use crate::components::status_badge::StatusBadge;
use crate::components::tag_badge::TagBadge;
use crate::server::audit::{create_comment, list_comments};
use crate::server::dns::list_dns_records;
use crate::server::ip_addresses::{delete_ip_address, get_ip_address, update_ip_address};

#[component]
pub fn IpDetailPage() -> impl IntoView {
    let params = use_params_map();
    let id = move || {
        params
            .read()
            .get("id")
            .and_then(|id| Uuid::parse_str(&id).ok())
    };

    let ip_data = Resource::new(id, |id| async move {
        match id {
            Some(id) => get_ip_address(id).await,
            None => Err(ServerFnError::new("Invalid IP ID")),
        }
    });

    let dns_records = Resource::new(id, |id| async move {
        match id {
            Some(id) => list_dns_records(Some(id)).await,
            None => Ok(Vec::new()),
        }
    });

    let comments = Resource::new(id, |id| async move {
        match id {
            Some(id) => list_comments("ip_address".to_string(), id).await,
            None => Ok(Vec::new()),
        }
    });

    let (editing, set_editing) = signal(false);
    let (edit_status, set_edit_status) = signal(String::new());
    let (edit_hostname, set_edit_hostname) = signal(String::new());
    let (edit_assigned_to, set_edit_assigned_to) = signal(String::new());
    let (edit_description, set_edit_description) = signal(String::new());
    let (edit_mac, set_edit_mac) = signal(String::new());

    let update_action = Action::new(
        move |input: &(Uuid, String, String, String, String, String)| {
            let (id, status, hostname, assigned_to, description, mac) = input.clone();
            let hostname = if hostname.is_empty() {
                None
            } else {
                Some(hostname)
            };
            let assigned_to = if assigned_to.is_empty() {
                None
            } else {
                Some(assigned_to)
            };
            let description = if description.is_empty() {
                None
            } else {
                Some(description)
            };
            let mac = if mac.is_empty() { None } else { Some(mac) };
            async move {
                let result =
                    update_ip_address(id, status, hostname, assigned_to, description, mac).await;
                if result.is_ok() {
                    set_editing.set(false);
                }
                result
            }
        },
    );

    let delete_action = Action::new(move |id: &Uuid| {
        let id = *id;
        async move { delete_ip_address(id).await }
    });

    let (new_comment, set_new_comment) = signal(String::new());
    let comment_action = Action::new(move |input: &(Uuid, String)| {
        let (id, content) = input.clone();
        async move {
            let result = create_comment("ip_address".to_string(), id, content).await;
            if result.is_ok() {
                set_new_comment.set(String::new());
            }
            result
        }
    });

    view! {
        <div>
            <a href="/ips" class="text-blue-400 hover:text-blue-300 text-sm mb-4 inline-block">
                "\u{2190} Back to IP Addresses"
            </a>

            <Suspense fallback=|| view! { <div class="text-slate-400">"Loading..."</div> }>
                {move || ip_data.get().map(|result| match result {
                    Ok(data) => {
                        let ip = data.ip.clone();
                        let ip_id = ip.id;
                        let address = ip.address.clone();
                        let status = ip.status.clone();
                        let subnet_cidr = data.subnet_cidr.clone();
                        let hostname_display = ip.hostname.clone().unwrap_or_else(|| "\u{2014}".to_string());
                        let assigned_display = ip.assigned_to.clone().unwrap_or_else(|| "\u{2014}".to_string());
                        let mac_display = ip.mac_address.clone().unwrap_or_else(|| "\u{2014}".to_string());
                        let created_display = ip.created_at.format("%Y-%m-%d %H:%M").to_string();
                        let tags = data.tags.clone();

                        let ip_for_edit = ip.clone();

                        view! {
                            <div class="mb-6">
                                <div class="flex justify-between items-start">
                                    <div>
                                        <h1 class="text-2xl font-bold text-white font-mono">{address}</h1>
                                        <div class="flex items-center gap-3 mt-2">
                                            <StatusBadge status=status />
                                            <span class="text-slate-400 text-sm font-mono">{subnet_cidr}</span>
                                        </div>
                                    </div>
                                    <div class="flex gap-2">
                                        <button
                                            on:click=move |_| {
                                                set_editing.set(true);
                                                set_edit_status.set(ip_for_edit.status.clone());
                                                set_edit_hostname.set(ip_for_edit.hostname.clone().unwrap_or_default());
                                                set_edit_assigned_to.set(ip_for_edit.assigned_to.clone().unwrap_or_default());
                                                set_edit_description.set(ip_for_edit.description.clone().unwrap_or_default());
                                                set_edit_mac.set(ip_for_edit.mac_address.clone().unwrap_or_default());
                                            }
                                            class="text-blue-400 hover:text-blue-300 text-sm px-3 py-1 border border-blue-400/30 rounded-lg">
                                            "Edit"
                                        </button>
                                        <button
                                            on:click=move |_| { delete_action.dispatch(ip_id); }
                                            class="text-red-400 hover:text-red-300 text-sm px-3 py-1 border border-red-400/30 rounded-lg">
                                            "Delete"
                                        </button>
                                    </div>
                                </div>
                            </div>

                            {move || editing.get().then(|| view! {
                                <div class="bg-slate-800 border border-slate-700 rounded-xl p-5 mb-6">
                                    <h2 class="text-lg font-semibold text-white mb-4">"Edit IP Address"</h2>
                                    <form on:submit=move |ev: leptos::ev::SubmitEvent| {
                                        ev.prevent_default();
                                        update_action.dispatch((
                                            ip_id,
                                            edit_status.get(),
                                            edit_hostname.get(),
                                            edit_assigned_to.get(),
                                            edit_description.get(),
                                            edit_mac.get(),
                                        ));
                                    } class="space-y-3">
                                        <div class="grid grid-cols-1 md:grid-cols-2 gap-3">
                                            <div>
                                                <label class="block text-xs text-slate-400 mb-1">"Status"</label>
                                                <select
                                                    prop:value=move || edit_status.get()
                                                    on:change=move |ev: leptos::ev::Event| set_edit_status.set(event_target_value(&ev))
                                                    class="w-full bg-slate-900 border border-slate-600 rounded px-3 py-1.5 text-white text-sm">
                                                    <option value="available">"Available"</option>
                                                    <option value="assigned">"Assigned"</option>
                                                    <option value="reserved">"Reserved"</option>
                                                    <option value="deprecated">"Deprecated"</option>
                                                </select>
                                            </div>
                                            <div>
                                                <label class="block text-xs text-slate-400 mb-1">"Hostname"</label>
                                                <input type="text"
                                                    prop:value=move || edit_hostname.get()
                                                    on:input=move |ev| set_edit_hostname.set(event_target_value(&ev))
                                                    class="w-full bg-slate-900 border border-slate-600 rounded px-3 py-1.5 text-white text-sm" />
                                            </div>
                                            <div>
                                                <label class="block text-xs text-slate-400 mb-1">"Assigned To"</label>
                                                <input type="text"
                                                    prop:value=move || edit_assigned_to.get()
                                                    on:input=move |ev| set_edit_assigned_to.set(event_target_value(&ev))
                                                    class="w-full bg-slate-900 border border-slate-600 rounded px-3 py-1.5 text-white text-sm" />
                                            </div>
                                            <div>
                                                <label class="block text-xs text-slate-400 mb-1">"MAC Address"</label>
                                                <input type="text"
                                                    prop:value=move || edit_mac.get()
                                                    on:input=move |ev| set_edit_mac.set(event_target_value(&ev))
                                                    class="w-full bg-slate-900 border border-slate-600 rounded px-3 py-1.5 text-white text-sm" />
                                            </div>
                                        </div>
                                        <div>
                                            <label class="block text-xs text-slate-400 mb-1">"Description"</label>
                                            <input type="text"
                                                prop:value=move || edit_description.get()
                                                on:input=move |ev| set_edit_description.set(event_target_value(&ev))
                                                class="w-full bg-slate-900 border border-slate-600 rounded px-3 py-1.5 text-white text-sm" />
                                        </div>
                                        <div class="flex gap-2">
                                            <button type="submit"
                                                class="bg-blue-500 hover:bg-blue-600 text-white px-3 py-1.5 rounded text-sm">
                                                "Save"
                                            </button>
                                            <button type="button"
                                                on:click=move |_| set_editing.set(false)
                                                class="bg-slate-700 hover:bg-slate-600 text-white px-3 py-1.5 rounded text-sm">
                                                "Cancel"
                                            </button>
                                        </div>
                                    </form>
                                </div>
                            })}

                            <div class="grid grid-cols-1 md:grid-cols-2 gap-4 mb-6">
                                <InfoCard label="Hostname" value=hostname_display />
                                <InfoCard label="Assigned To" value=assigned_display />
                                <InfoCard label="MAC Address" value=mac_display />
                                <InfoCard label="Created" value=created_display />
                            </div>

                            {(!tags.is_empty()).then(|| {
                                let tags_owned = tags;
                                view! {
                                    <div class="mb-6">
                                        <h3 class="text-sm font-medium text-slate-400 mb-2">"Tags"</h3>
                                        <div class="flex gap-2 flex-wrap">
                                            {tags_owned.into_iter().map(|tag| view! {
                                                <TagBadge tag=tag />
                                            }).collect::<Vec<_>>()}
                                        </div>
                                    </div>
                                }
                            })}

                            <div class="bg-slate-800 border border-slate-700 rounded-xl p-5 mb-6">
                                <h2 class="text-lg font-semibold text-white mb-4">"DNS Records"</h2>
                                <Suspense fallback=|| view! { <div class="text-slate-400">"Loading..."</div> }>
                                    {move || dns_records.get().map(|result| match result {
                                        Ok(records) => {
                                            if records.is_empty() {
                                                view! { <p class="text-slate-500 text-sm">"No DNS records"</p> }.into_any()
                                            } else {
                                                view! {
                                                    <div class="space-y-2">
                                                        {records.into_iter().map(|r| {
                                                            let rtype = r.record_type.clone();
                                                            let rhost = r.hostname.clone();
                                                            view! {
                                                                <div class="flex items-center gap-3 py-2 border-b border-slate-700 last:border-0">
                                                                    <span class="text-xs bg-slate-700 px-2 py-0.5 rounded font-mono">{rtype}</span>
                                                                    <span class="text-white">{rhost}</span>
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

                            <div class="bg-slate-800 border border-slate-700 rounded-xl p-5">
                                <h2 class="text-lg font-semibold text-white mb-4">"Comments"</h2>

                                <form on:submit=move |ev: leptos::ev::SubmitEvent| {
                                    ev.prevent_default();
                                    comment_action.dispatch((ip_id, new_comment.get()));
                                } class="flex gap-2 mb-4">
                                    <input type="text"
                                        placeholder="Add a comment..."
                                        prop:value=move || new_comment.get()
                                        on:input=move |ev| set_new_comment.set(event_target_value(&ev))
                                        class="flex-1 bg-slate-900 border border-slate-600 rounded px-3 py-1.5 text-white text-sm" />
                                    <button type="submit"
                                        class="bg-blue-500 hover:bg-blue-600 text-white px-3 py-1.5 rounded text-sm">
                                        "Post"
                                    </button>
                                </form>

                                <Suspense fallback=|| view! { <div class="text-slate-400">"Loading..."</div> }>
                                    {move || comments.get().map(|result| match result {
                                        Ok(list) => {
                                            if list.is_empty() {
                                                view! { <p class="text-slate-500 text-sm">"No comments yet"</p> }.into_any()
                                            } else {
                                                view! {
                                                    <div class="space-y-3">
                                                        {list.into_iter().map(|c| {
                                                            let content = c.content.clone();
                                                            let time = c.created_at.format("%Y-%m-%d %H:%M").to_string();
                                                            view! {
                                                                <div class="border-b border-slate-700 pb-3 last:border-0">
                                                                    <p class="text-slate-300 text-sm">{content}</p>
                                                                    <span class="text-slate-500 text-xs">{time}</span>
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
                        }.into_any()
                    },
                    Err(e) => view! { <p class="text-red-400">{e.to_string()}</p> }.into_any(),
                })}
            </Suspense>
        </div>
    }
}

#[component]
fn InfoCard(#[prop(into)] label: String, #[prop(into)] value: String) -> impl IntoView {
    view! {
        <div class="bg-slate-800 border border-slate-700 rounded-lg p-4">
            <p class="text-slate-400 text-xs mb-1">{label}</p>
            <p class="text-white text-sm">{value}</p>
        </div>
    }
}
