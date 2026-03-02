use leptos::prelude::*;
use leptos_router::hooks::use_params_map;
use uuid::Uuid;

use crate::components::utilization_bar::UtilizationBar;
use crate::server::vlans::{delete_vlan, get_vlan};

#[component]
pub fn VlanDetailPage() -> impl IntoView {
    let params = use_params_map();
    let id = move || {
        params
            .read()
            .get("id")
            .and_then(|id| Uuid::parse_str(&id).ok())
    };

    let vlan_data = Resource::new(id, |id| async move {
        match id {
            Some(id) => get_vlan(id).await,
            None => Err(ServerFnError::new("Invalid VLAN ID")),
        }
    });

    let delete_action = Action::new(move |id: &Uuid| {
        let id = *id;
        async move { delete_vlan(id).await }
    });

    view! {
        <div>
            <a href="/vlans" class="text-blue-400 hover:text-blue-300 text-sm mb-4 inline-block">
                "\u{2190} Back to VLANs"
            </a>

            <Suspense fallback=|| view! { <div class="text-slate-400">"Loading..."</div> }>
                {move || vlan_data.get().map(|result| match result {
                    Ok(data) => {
                        let vlan = data.vlan;
                        let vlan_id_val = vlan.id;
                        let title = format!("VLAN {} \u{2014} {}", vlan.vlan_id, vlan.name);
                        let desc = vlan.description.clone();

                        view! {
                            <div class="mb-6">
                                <div class="flex justify-between items-start">
                                    <div>
                                        <h1 class="text-2xl font-bold text-white">{title}</h1>
                                        {desc.map(|d| view! {
                                            <p class="text-slate-400 mt-2">{d}</p>
                                        })}
                                    </div>
                                    <button
                                        on:click=move |_| { delete_action.dispatch(vlan_id_val); }
                                        class="text-red-400 hover:text-red-300 text-sm px-3 py-1 border border-red-400/30 rounded-lg">
                                        "Delete"
                                    </button>
                                </div>
                            </div>

                            <div class="bg-slate-800 border border-slate-700 rounded-xl p-5">
                                <h2 class="text-lg font-semibold text-white mb-4">"Linked Subnets"</h2>
                                {if data.subnets.is_empty() {
                                    view! { <p class="text-slate-500 text-sm">"No subnets linked to this VLAN"</p> }.into_any()
                                } else {
                                    view! {
                                        <div class="space-y-3">
                                            {data.subnets.into_iter().map(|subnet| {
                                                let href = format!("/subnets/{}", subnet.id);
                                                let sname = subnet.name.clone();
                                                let scidr = subnet.cidr.clone();
                                                view! {
                                                    <a href=href class="block bg-slate-900 border border-slate-700 rounded-lg p-4 hover:border-blue-500/50">
                                                        <div class="flex justify-between items-center mb-2">
                                                            <div>
                                                                <span class="text-white font-medium">{sname}</span>
                                                                " "
                                                                <span class="text-blue-400 font-mono text-sm">{scidr}</span>
                                                            </div>
                                                        </div>
                                                        <UtilizationBar used=subnet.used_count total=subnet.total_addresses />
                                                    </a>
                                                }
                                            }).collect::<Vec<_>>()}
                                        </div>
                                    }.into_any()
                                }}
                            </div>
                        }.into_any()
                    },
                    Err(e) => view! { <p class="text-red-400">{e.to_string()}</p> }.into_any(),
                })}
            </Suspense>
        </div>
    }
}
