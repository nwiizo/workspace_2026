use leptos::prelude::*;

use crate::components::utilization_bar::UtilizationBar;
use crate::models::subnet::Subnet;

#[component]
pub fn SubnetCard(subnet: Subnet) -> impl IntoView {
    let id = subnet.id.to_string();
    let href = format!("/subnets/{}", id);
    let name = subnet.name.clone();
    let cidr = subnet.cidr.clone();
    let short_id = id.chars().take(8).collect::<String>();
    let desc = subnet.description.clone();

    view! {
        <a href=href
           class="block bg-slate-800 border border-slate-700 rounded-xl p-5 hover:border-blue-500/50 transition-colors">
            <div class="flex justify-between items-start mb-3">
                <div>
                    <h3 class="text-white font-semibold text-lg">{name}</h3>
                    <p class="text-blue-400 font-mono text-sm mt-1">{cidr}</p>
                </div>
                <span class="text-xs text-slate-500 bg-slate-700/50 px-2 py-1 rounded">
                    {short_id}
                </span>
            </div>

            {desc.map(|d| view! {
                <p class="text-slate-400 text-sm mb-3 line-clamp-2">{d}</p>
            })}

            <UtilizationBar used=subnet.used_count total=subnet.total_addresses />
        </a>
    }
}
