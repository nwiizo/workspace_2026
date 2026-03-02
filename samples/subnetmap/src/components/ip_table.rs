use leptos::prelude::*;

use crate::components::status_badge::StatusBadge;
use crate::components::tag_badge::TagBadge;
use crate::models::ip_address::IpAddressWithMeta;

#[component]
pub fn IpTable(ips: Vec<IpAddressWithMeta>) -> impl IntoView {
    view! {
        <div class="overflow-x-auto">
            <table class="w-full text-sm">
                <thead>
                    <tr class="border-b border-slate-700 text-slate-400 text-left">
                        <th class="pb-3 pr-4 font-medium">"Address"</th>
                        <th class="pb-3 pr-4 font-medium">"Hostname"</th>
                        <th class="pb-3 pr-4 font-medium">"Status"</th>
                        <th class="pb-3 pr-4 font-medium">"Assigned To"</th>
                        <th class="pb-3 pr-4 font-medium">"Subnet"</th>
                        <th class="pb-3 font-medium">"Tags"</th>
                    </tr>
                </thead>
                <tbody>
                    {ips.into_iter().map(|entry| {
                        let href = format!("/ips/{}", entry.ip.id);
                        let address = entry.ip.address.clone();
                        let hostname = entry.ip.hostname.clone().unwrap_or_else(|| "\u{2014}".to_string());
                        let status = entry.ip.status.clone();
                        let assigned_to = entry.ip.assigned_to.clone().unwrap_or_else(|| "\u{2014}".to_string());
                        let subnet_cidr = entry.subnet_cidr.clone();
                        let tags = entry.tags;
                        view! {
                            <tr class="border-b border-slate-800 hover:bg-slate-800/50">
                                <td class="py-3 pr-4">
                                    <a href=href class="text-blue-400 hover:text-blue-300 font-mono">
                                        {address}
                                    </a>
                                </td>
                                <td class="py-3 pr-4 text-slate-300">{hostname}</td>
                                <td class="py-3 pr-4">
                                    <StatusBadge status=status />
                                </td>
                                <td class="py-3 pr-4 text-slate-300">{assigned_to}</td>
                                <td class="py-3 pr-4 text-slate-400 font-mono text-xs">{subnet_cidr}</td>
                                <td class="py-3">
                                    <div class="flex gap-1 flex-wrap">
                                        {tags.into_iter().map(|tag| view! {
                                            <TagBadge tag=tag />
                                        }).collect::<Vec<_>>()}
                                    </div>
                                </td>
                            </tr>
                        }
                    }).collect::<Vec<_>>()}
                </tbody>
            </table>
        </div>
    }
}
