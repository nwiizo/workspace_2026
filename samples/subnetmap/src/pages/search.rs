use leptos::prelude::*;

use crate::server::search::global_search;

#[component]
pub fn SearchPage() -> impl IntoView {
    let (query, set_query) = signal(String::new());

    let results = Resource::new(
        move || query.get(),
        |q| async move {
            if q.trim().is_empty() {
                Ok(Vec::new())
            } else {
                global_search(q).await
            }
        },
    );

    view! {
        <div>
            <h1 class="text-2xl font-bold text-white mb-6">"Search"</h1>

            <div class="mb-6">
                <input type="text"
                    placeholder="Search subnets, IPs, VLANs, DNS records..."
                    prop:value=move || query.get()
                    on:input=move |ev| set_query.set(event_target_value(&ev))
                    class="w-full bg-slate-800 border border-slate-600 rounded-xl px-4 py-3 text-white focus:border-blue-500 focus:outline-none" />
            </div>

            <Suspense fallback=|| view! { <div class="text-slate-400">"Searching..."</div> }>
                {move || results.get().map(|result| match result {
                    Ok(list) => {
                        if query.get().trim().is_empty() {
                            view! {
                                <div class="text-center py-12 text-slate-500">
                                    <p>"Enter a search term to find subnets, IPs, VLANs, and DNS records"</p>
                                </div>
                            }.into_any()
                        } else if list.is_empty() {
                            view! {
                                <div class="text-center py-12 text-slate-500">
                                    <p>"No results found"</p>
                                </div>
                            }.into_any()
                        } else {
                            view! {
                                <div class="space-y-2">
                                    {list.into_iter().map(|item| {
                                        let href = match item.entity_type.as_str() {
                                            "subnet" => format!("/subnets/{}", item.id),
                                            "ip_address" => format!("/ips/{}", item.id),
                                            "vlan" => format!("/vlans/{}", item.id),
                                            _ => "#".to_string(),
                                        };
                                        let type_badge_class = match item.entity_type.as_str() {
                                            "subnet" => "bg-blue-500/20 text-blue-400",
                                            "ip_address" => "bg-green-500/20 text-green-400",
                                            "vlan" => "bg-purple-500/20 text-purple-400",
                                            "dns_record" => "bg-cyan-500/20 text-cyan-400",
                                            _ => "bg-slate-500/20 text-slate-400",
                                        };
                                        let badge_class = format!("text-xs px-2 py-0.5 rounded {}", type_badge_class);
                                        let entity_type = item.entity_type.clone();
                                        let title = item.title.clone();
                                        let desc = item.description.clone();
                                        let has_desc = !desc.is_empty();

                                        view! {
                                            <a href=href
                                               class="block bg-slate-800 border border-slate-700 rounded-lg p-4 hover:border-blue-500/50 transition-colors">
                                                <div class="flex items-center gap-3">
                                                    <span class=badge_class>{entity_type}</span>
                                                    <span class="text-white font-medium">{title}</span>
                                                </div>
                                                {has_desc.then(|| view! {
                                                    <p class="text-slate-400 text-sm mt-1 ml-16">{desc}</p>
                                                })}
                                            </a>
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
    }
}
